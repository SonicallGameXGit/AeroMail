//! Message HTML is untrusted input. It is cleaned here and additionally shown
//! by the UI in an isolated frame that cannot run scripts.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use ammonia::Builder;

pub(crate) struct Sanitised {
    pub html: String,
    /// True when remote resources were removed (or are present in CSS, where
    /// the frame's content policy blocks them).
    pub remote_blocked: bool,
}

fn is_remote(value: &str) -> bool {
    let value = value.trim_start().to_ascii_lowercase();

    value.starts_with("http:") || value.starts_with("https:") || value.starts_with("//")
}

/// `inline_images` maps Content-IDs to data URIs for `cid:` references.
pub(crate) fn sanitise(
    html: &str,
    inline_images: &HashMap<String, String>,
    allow_remote: bool,
) -> Sanitised {
    // The filter must be 'static, so it gets its own copies.
    let removed_remote = Arc::new(AtomicBool::new(false));
    let removed_remote_flag = Arc::clone(&removed_remote);
    let inline_images = inline_images.clone();

    let mut builder = Builder::default();
    builder
        // Layout of real-world mail depends on these presentational bits.
        .add_tags(["font", "center", "style"])
        .rm_clean_content_tags(["style"])
        .add_generic_attributes([
            "style",
            "align",
            "valign",
            "bgcolor",
            "color",
            "width",
            "height",
            "border",
            "cellpadding",
            "cellspacing",
            "dir",
        ])
        .add_tag_attributes("font", ["face", "size"])
        .add_tag_attributes("td", ["background"])
        .add_tag_attributes("table", ["background"])
        .url_schemes(HashSet::from(["http", "https", "mailto", "cid", "data"]))
        .link_rel(Some("noopener noreferrer"))
        .attribute_filter(move |element, attribute, value| {
            let loads_resource = matches!(
                (element, attribute),
                ("img", "src") | ("img", "srcset") | (_, "background")
            );
            if !loads_resource {
                return Some(value.into());
            }

            if let Some(content_id) = value.strip_prefix("cid:") {
                return inline_images.get(content_id).map(|uri| uri.clone().into());
            }

            if is_remote(value) || attribute == "srcset" {
                if allow_remote {
                    return Some(value.into());
                }
                removed_remote_flag.store(true, Ordering::Relaxed);

                return None;
            }

            Some(value.into())
        });

    let clean = builder.clean(html).to_string();
    let css_remote =
        clean.contains("url(") && (clean.contains("http:") || clean.contains("https:"));
    let remote_blocked = !allow_remote && (removed_remote.load(Ordering::Relaxed) || css_remote);

    Sanitised {
        html: clean,
        remote_blocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean(html: &str, allow_remote: bool) -> Sanitised {
        sanitise(html, &HashMap::new(), allow_remote)
    }

    #[test]
    fn removes_scripts_and_event_handlers() {
        let result = clean(
            "<p onclick=\"steal()\">Hi</p><script>steal()</script><iframe src=\"https://evil.example\"></iframe>",
            true,
        );

        assert_eq!(result.html, "<p>Hi</p>");
    }

    #[test]
    fn removes_javascript_links() {
        let result = clean("<a href=\"javascript:steal()\">x</a>", true);

        assert!(!result.html.contains("javascript"));
    }

    #[test]
    fn keeps_presentational_markup() {
        let result = clean(
            "<table bgcolor=\"#fff\" width=\"600\"><tr><td style=\"color: red\" align=\"center\">x</td></tr></table>",
            false,
        );

        assert!(result.html.contains("bgcolor=\"#fff\""));
        assert!(result.html.contains("style=\"color: red\""));
        assert!(!result.remote_blocked);
    }

    #[test]
    fn blocks_remote_images_until_allowed() {
        let html = "<img src=\"https://tracker.example/pixel.gif\" alt=\"logo\">";

        let blocked = clean(html, false);
        assert!(!blocked.html.contains("tracker.example"));
        assert!(blocked.html.contains("alt=\"logo\""));
        assert!(blocked.remote_blocked);

        let allowed = clean(html, true);
        assert!(allowed.html.contains("https://tracker.example/pixel.gif"));
        assert!(!allowed.remote_blocked);
    }

    #[test]
    fn reports_remote_content_in_css() {
        let result = clean(
            "<div style=\"background: url(https://tracker.example/bg.png)\">x</div>",
            false,
        );

        assert!(result.remote_blocked);
    }

    #[test]
    fn inline_images_are_embedded() {
        let images = HashMap::from([(
            "logo@mail".to_string(),
            "data:image/png;base64,AAAA".to_string(),
        )]);
        let result = sanitise(
            "<img src=\"cid:logo@mail\"><img src=\"cid:missing\">",
            &images,
            false,
        );

        assert!(result.html.contains("src=\"data:image/png;base64,AAAA\""));
        assert!(!result.html.contains("cid:"));
        assert!(!result.remote_blocked);
    }
}
