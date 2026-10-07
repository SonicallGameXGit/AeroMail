//! Message HTML is untrusted input. It is cleaned here and additionally shown
//! by the UI in an isolated frame that cannot run scripts.

use std::borrow::Cow;
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

/// What a blocked remote image points at instead: an image that is empty
/// without being missing. The `<img>` keeps its size and alt text, as in a
/// client that blocks the download, while an `<img>` with no address at all
/// is drawn by the browser as an empty box with a border.
const BLOCKED_IMAGE: &str = "data:,";

/// Where the first `<body` tag starts, outside comments. ASCII lowercasing
/// keeps byte positions, so they are valid in `html` too.
fn body_tag(html: &str) -> Option<usize> {
    let lower = html.to_ascii_lowercase();
    let mut from = 0;
    while let Some(found) = lower[from..].find("<body") {
        let start = from + found;
        from = start + "<body".len();

        let ends_name = matches!(
            lower.as_bytes().get(from),
            Some(b' ' | b'\t' | b'\r' | b'\n' | b'\x0c' | b'/' | b'>')
        );
        let in_comment = lower[..start]
            .rfind("<!--")
            .is_some_and(|comment| !lower[comment..start].contains("-->"));
        if ends_name && !in_comment {
            return Some(start);
        }
    }

    None
}

/// An attribute as written: its name, its value without the quotes, and the
/// quote it was written with (`"` for an unquoted one).
type Attribute<'a> = (&'a str, &'a str, char);

/// The attributes of the tag whose name ends at `from`, and where the tag
/// ends (after its `>`).
fn attributes(html: &str, mut from: usize) -> (Vec<Attribute<'_>>, usize) {
    let bytes = html.as_bytes();
    let space = |byte: u8| byte.is_ascii_whitespace() || byte == b'/';
    let mut found = Vec::new();

    loop {
        while bytes.get(from).is_some_and(|byte| space(*byte)) {
            from += 1;
        }
        match bytes.get(from) {
            None => return (found, from),
            Some(b'>') => return (found, from + 1),
            Some(_) => {}
        }

        let name_start = from;
        while bytes
            .get(from)
            .is_some_and(|byte| !space(*byte) && *byte != b'=' && *byte != b'>')
        {
            from += 1;
        }
        let name = &html[name_start..from];
        while bytes.get(from).is_some_and(u8::is_ascii_whitespace) {
            from += 1;
        }
        if bytes.get(from) != Some(&b'=') {
            found.push((name, "", '"'));
            continue;
        }
        from += 1;
        while bytes.get(from).is_some_and(u8::is_ascii_whitespace) {
            from += 1;
        }

        match bytes.get(from) {
            Some(quote @ (b'"' | b'\'')) => {
                let value_start = from + 1;
                let value_end = html[value_start..]
                    .find(char::from(*quote))
                    .map_or(html.len(), |end| value_start + end);
                found.push((name, &html[value_start..value_end], char::from(*quote)));
                from = (value_end + 1).min(html.len());
            }
            _ => {
                let value_start = from;
                while bytes
                    .get(from)
                    .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>')
                {
                    from += 1;
                }
                found.push((name, &html[value_start..from], '"'));
            }
        }
    }
}

/// A colour from a presentational attribute, if it is only a colour: it is
/// copied into a style, where anything else could add more rules.
fn plain_colour(value: &str) -> Option<&str> {
    let value = value.trim();
    let plain = !value.is_empty() && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '#');

    plain.then_some(value)
}

/// Mail sets its page's background, margins, font and colour on `<body>`,
/// but the sanitiser works on a fragment, where `<body>` is dropped with all
/// of that. So the tag is taken out first, its attributes are written on a
/// `<div>` to be sanitised like any other, and that becomes the page's body
/// again (`as_body`). `text`, which only a body understands, moves into its
/// style along with `bgcolor`, before the message's own style so that wins.
///
/// Gives the stand-in `<div>` and the message without its body tag.
fn take_body(html: &str) -> (Option<String>, Cow<'_, str>) {
    let Some(start) = body_tag(html) else {
        return (None, Cow::Borrowed(html));
    };
    let (found, end) = attributes(html, start + "<body".len());

    let mut hints = String::new();
    let mut style = None;
    let mut kept = String::new();
    for (name, value, quote) in found {
        match name.to_ascii_lowercase().as_str() {
            "bgcolor" => {
                if let Some(colour) = plain_colour(value) {
                    hints.push_str(&format!("background-color:{colour};"));
                }
            }
            "text" => {
                if let Some(colour) = plain_colour(value) {
                    hints.push_str(&format!("color:{colour};"));
                }
            }
            // The first one counts, as in a browser.
            "style" if style.is_none() => style = Some((value, quote)),
            _ => kept.push_str(&format!(" {name}={quote}{value}{quote}")),
        }
    }
    let (own, quote) = style.unwrap_or(("", '"'));
    let style = if hints.is_empty() && own.is_empty() {
        String::new()
    } else {
        format!(" style={quote}{hints}{own}{quote}")
    };

    let rest = format!("{}{}", &html[..start], &html[end..]);

    (Some(format!("<div{style}{kept}></div>")), Cow::Owned(rest))
}

/// The sanitised stand-in `<div>` as a `<body>` tag. The frame puts the
/// message right after its own `<body>`, and a browser merges a second body
/// tag's attributes into the first: the message's body styles then override
/// the frame's defaults exactly as they would a browser's.
fn as_body(clean_div: &str) -> Option<String> {
    let attributes = clean_div.strip_prefix("<div")?.strip_suffix("></div>")?;

    Some(format!("<body{attributes}>"))
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
        // The page title is not part of the message; without this its text
        // would be left behind at the top of the body.
        .add_clean_content_tags(["title"])
        .add_generic_attributes([
            // The message's own style sheet is written against these. They
            // are harmless inside the frame: it runs no scripts and holds
            // nothing but the message.
            "class",
            "id",
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
            "hspace",
            "vspace",
            "nowrap",
            "role",
            "dir",
        ])
        .add_tag_attributes("font", ["face", "size"])
        // Without it, a style sheet meant for one screen size applies to all.
        .add_tag_attributes("style", ["media"])
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

                return (attribute == "src").then(|| BLOCKED_IMAGE.into());
            }

            Some(value.into())
        });

    let (body, rest) = take_body(html);
    let body = body
        .and_then(|div| as_body(&builder.clean(&div).to_string()))
        .unwrap_or_default();
    let clean = body + &builder.clean(&rest).to_string();
    let css_remote = (clean.contains("url(") || clean.contains("@import"))
        && (clean.contains("http:") || clean.contains("https:"));
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
    fn keeps_what_the_style_sheet_selects_by() {
        let result = clean(
            "<style>.pad{padding:24px 32px}#top{font-size:22px}</style>\
             <table><tr><td class=\"pad\" id=\"top\">x</td></tr></table>",
            false,
        );

        assert!(result.html.contains(".pad{padding:24px 32px}"));
        assert!(result.html.contains("class=\"pad\""));
        assert!(result.html.contains("id=\"top\""));
    }

    #[test]
    fn the_page_style_of_the_body_is_kept() {
        let result = clean(
            "<html><head><style>p{margin:0}</style></head>\
             <body bgcolor=\"#f5f7fa\" text=#111 style='margin:0;font-family:\"Inter\"' class=\"body\">\
             <p>Hi</p></body></html>",
            false,
        );

        assert_eq!(
            result.html,
            "<body style=\"background-color:#f5f7fa;color:#111;margin:0;font-family:&quot;Inter&quot;\" class=\"body\">\
             <style>p{margin:0}</style><p>Hi</p>"
        );
    }

    #[test]
    fn only_a_real_body_tag_is_turned_into_a_block() {
        let result = clean(
            "<!-- <body bgcolor=red> --><bodyguard>x</bodyguard><body bgcolor=\"red;background:url(x)\">y</body>",
            false,
        );

        assert_eq!(result.html, "<body>xy");
    }

    #[test]
    fn the_body_tag_is_sanitised_like_any_other() {
        let result = clean(
            "<body onload=\"steal()\" style=\"margin:0\" background=\"https://tracker.example/bg.png\">x</body>",
            false,
        );

        assert_eq!(result.html, "<body style=\"margin:0\">x");
    }

    #[test]
    fn a_style_sheet_keeps_the_screen_it_is_meant_for() {
        let result = clean(
            "<style media=\"(max-width:480px)\">td{display:block}</style>",
            false,
        );

        assert!(result.html.contains("media=\"(max-width:480px)\""));
    }

    #[test]
    fn page_title_is_not_shown_as_text() {
        let result = clean(
            "<html><head><title>Shop\n</title></head><body><p>Hi</p></body></html>",
            false,
        );

        assert!(!result.html.contains("Shop"));
        assert!(result.html.contains("<p>Hi</p>"));
    }

    #[test]
    fn blocks_remote_images_until_allowed() {
        let html = "<img src=\"https://tracker.example/pixel.gif\" alt=\"logo\">";

        let blocked = clean(html, false);
        assert!(!blocked.html.contains("tracker.example"));
        assert!(blocked.html.contains("alt=\"logo\""));
        assert!(blocked.html.contains("src=\"data:,\""));
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
    fn reports_imported_style_sheets_and_fonts() {
        let imported = clean(
            "<style>@import \"https://fonts.example/css?family=Inter\";</style>x",
            false,
        );
        assert!(imported.remote_blocked);

        let font = clean(
            "<style>@font-face{font-family:Brand;src:url('https://cdn.example/brand.woff2')}</style>x",
            false,
        );
        assert!(font.remote_blocked);
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
