//! What the user can change about how the app behaves, as one value.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{ErrorKind, MailError, MailResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppSettings {
    /// A brand's own logo is looked up for its mail. Off: nothing is asked.
    pub sender_pictures: bool,
    /// Remote images load in every message without asking for each one.
    pub remote_images: bool,
    pub notifications: NotificationSettings,
}

/// How long a notification stays on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum NotificationDuration {
    /// As long as the system shows notifications.
    System,
    /// `NotificationSettings::seconds`, then it is taken away.
    Custom,
    /// Until the user closes it (where the system allows it).
    UntilDismissed,
}

/// What stands next to the text of a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum NotificationIcon {
    /// The sender's logo, or their coloured initial, as in the message list.
    Sender,
    App,
    /// `NotificationSettings::custom_icon`.
    Custom,
    None,
}

/// The notification shown when new mail arrives in an inbox.
///
/// `title` and `body` are templates: `{sender}`, `{address}`, `{subject}`,
/// `{preview}` and `{account}` are replaced by what the message has (the UI
/// does it, in `features/notifications`); everything else is shown as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NotificationSettings {
    /// Off: no notification at all. Mail still arrives in the lists.
    pub enabled: bool,
    pub sound: bool,
    /// Also shown while the app's window is in front.
    pub while_focused: bool,
    pub duration: NotificationDuration,
    /// Used when `duration` is `Custom`.
    pub seconds: u32,
    pub title: String,
    pub body: String,
    /// How many characters of the message `{preview}` shows.
    pub preview_length: u32,
    pub icon: NotificationIcon,
    /// A small PNG as a `data:` address, for `NotificationIcon::Custom`.
    pub custom_icon: Option<String>,
}

pub const SECONDS_RANGE: (u32, u32) = (1, 3600);
pub const PREVIEW_RANGE: (u32, u32) = (10, 500);
/// A template longer than this cannot be shown by any notification service.
pub const TEMPLATE_LIMIT: usize = 500;
/// Pictures are made small (in the UI) before they are kept.
pub const CUSTOM_ICON_LIMIT: usize = 256 * 1024;
const CUSTOM_ICON_PREFIX: &str = "data:image/png;base64,";

impl Default for AppSettings {
    /// What a fresh install does. Remote images stay blocked until the user
    /// decides otherwise: loading one tells the sender the mail was read.
    fn default() -> Self {
        Self {
            sender_pictures: true,
            remote_images: false,
            notifications: NotificationSettings::default(),
        }
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            sound: true,
            while_focused: false,
            duration: NotificationDuration::System,
            seconds: 10,
            title: "{sender}".to_string(),
            body: "{subject}\n{preview}".to_string(),
            preview_length: 90,
            icon: NotificationIcon::Sender,
            custom_icon: None,
        }
    }
}

impl NotificationSettings {
    /// Whether the message itself has to be downloaded for a notification.
    pub fn shows_preview(&self) -> bool {
        self.enabled && (self.title.contains("{preview}") || self.body.contains("{preview}"))
    }
}

fn invalid(detail: &str) -> MailError {
    MailError::new(ErrorKind::Invalid, detail)
}

fn is_custom_icon(icon: &str) -> bool {
    icon.starts_with(CUSTOM_ICON_PREFIX) && icon.len() <= CUSTOM_ICON_LIMIT
}

impl AppSettings {
    /// Refuses what no notification could show, rather than storing it.
    pub fn check(&self) -> MailResult<()> {
        let notifications = &self.notifications;
        let within = |value: u32, (low, high): (u32, u32)| (low..=high).contains(&value);

        if !within(notifications.seconds, SECONDS_RANGE) {
            return Err(invalid("the notification time is out of range"));
        }
        if !within(notifications.preview_length, PREVIEW_RANGE) {
            return Err(invalid("the preview length is out of range"));
        }
        if notifications.title.chars().count() > TEMPLATE_LIMIT
            || notifications.body.chars().count() > TEMPLATE_LIMIT
        {
            return Err(invalid("the notification text is too long"));
        }
        if let Some(icon) = &notifications.custom_icon
            && !is_custom_icon(icon)
        {
            return Err(invalid("the notification picture is not a small PNG"));
        }

        Ok(())
    }

    /// Reads the settings from stored name / value pairs. A missing or
    /// unreadable value has its default, so stored choices survive new
    /// fields and old versions.
    pub fn from_stored(stored: impl Fn(&str) -> Option<String>) -> Self {
        let default = Self::default();
        let base = &default.notifications;
        let switch = |name: &str, fallback: bool| match stored(name).as_deref() {
            Some("on") => true,
            Some("off") => false,
            _ => fallback,
        };
        let number = |name: &str, fallback: u32, (low, high): (u32, u32)| {
            stored(name)
                .and_then(|value| value.parse::<u32>().ok())
                .filter(|value| (low..=high).contains(value))
                .unwrap_or(fallback)
        };
        let text = |name: &str, fallback: &str| {
            stored(name)
                .filter(|value| value.chars().count() <= TEMPLATE_LIMIT)
                .unwrap_or_else(|| fallback.to_string())
        };

        let duration = match stored(DURATION).as_deref() {
            Some("custom") => NotificationDuration::Custom,
            Some("until_dismissed") => NotificationDuration::UntilDismissed,
            Some("system") => NotificationDuration::System,
            _ => base.duration,
        };
        let icon = match stored(ICON).as_deref() {
            Some("sender") => NotificationIcon::Sender,
            Some("app") => NotificationIcon::App,
            Some("custom") => NotificationIcon::Custom,
            Some("none") => NotificationIcon::None,
            _ => base.icon,
        };

        Self {
            sender_pictures: switch(SENDER_PICTURES, default.sender_pictures),
            remote_images: switch(REMOTE_IMAGES, default.remote_images),
            notifications: NotificationSettings {
                enabled: switch(NOTIFICATIONS, base.enabled),
                sound: switch(SOUND, base.sound),
                while_focused: switch(WHILE_FOCUSED, base.while_focused),
                duration,
                seconds: number(SECONDS, base.seconds, SECONDS_RANGE),
                title: text(TITLE, &base.title),
                body: text(BODY, &base.body),
                preview_length: number(PREVIEW_LENGTH, base.preview_length, PREVIEW_RANGE),
                icon,
                custom_icon: stored(CUSTOM_ICON).filter(|icon| is_custom_icon(icon)),
            },
        }
    }

    /// The settings as name / value pairs, the way `from_stored` reads them.
    /// `None` means the value is to be removed.
    pub fn to_stored(&self) -> Vec<(&'static str, Option<String>)> {
        let switch = |on: bool| Some(if on { "on" } else { "off" }.to_string());
        let notifications = &self.notifications;
        let duration = match notifications.duration {
            NotificationDuration::System => "system",
            NotificationDuration::Custom => "custom",
            NotificationDuration::UntilDismissed => "until_dismissed",
        };
        let icon = match notifications.icon {
            NotificationIcon::Sender => "sender",
            NotificationIcon::App => "app",
            NotificationIcon::Custom => "custom",
            NotificationIcon::None => "none",
        };

        vec![
            (SENDER_PICTURES, switch(self.sender_pictures)),
            (REMOTE_IMAGES, switch(self.remote_images)),
            (NOTIFICATIONS, switch(notifications.enabled)),
            (SOUND, switch(notifications.sound)),
            (WHILE_FOCUSED, switch(notifications.while_focused)),
            (DURATION, Some(duration.to_string())),
            (SECONDS, Some(notifications.seconds.to_string())),
            (TITLE, Some(notifications.title.clone())),
            (BODY, Some(notifications.body.clone())),
            (
                PREVIEW_LENGTH,
                Some(notifications.preview_length.to_string()),
            ),
            (ICON, Some(icon.to_string())),
            (CUSTOM_ICON, notifications.custom_icon.clone()),
        ]
    }
}

// Names of the stored values. "notifications" is the on / off switch it has
// been since before the other notification settings existed.
const SENDER_PICTURES: &str = "sender_pictures";
const REMOTE_IMAGES: &str = "remote_images";
const NOTIFICATIONS: &str = "notifications";
const SOUND: &str = "notification_sound";
const WHILE_FOCUSED: &str = "notification_while_focused";
const DURATION: &str = "notification_duration";
const SECONDS: &str = "notification_seconds";
const TITLE: &str = "notification_title";
const BODY: &str = "notification_body";
const PREVIEW_LENGTH: &str = "notification_preview_length";
const ICON: &str = "notification_icon";
const CUSTOM_ICON: &str = "notification_custom_icon";

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn round_trip(settings: &AppSettings) -> AppSettings {
        let stored: HashMap<&str, String> = settings
            .to_stored()
            .into_iter()
            .filter_map(|(name, value)| Some((name, value?)))
            .collect();

        AppSettings::from_stored(|name| stored.get(name).cloned())
    }

    #[test]
    fn nothing_stored_is_the_default() {
        assert_eq!(AppSettings::from_stored(|_| None), AppSettings::default());
    }

    #[test]
    fn every_choice_is_read_back_as_written() {
        let settings = AppSettings {
            sender_pictures: true,
            remote_images: true,
            notifications: NotificationSettings {
                enabled: false,
                sound: false,
                while_focused: true,
                duration: NotificationDuration::Custom,
                seconds: 42,
                title: "{subject}".to_string(),
                body: "From {address}".to_string(),
                preview_length: 120,
                icon: NotificationIcon::Custom,
                custom_icon: Some("data:image/png;base64,AAAA".to_string()),
            },
        };

        assert_eq!(round_trip(&settings), settings);
    }

    #[test]
    fn values_that_make_no_sense_fall_back_to_the_default() {
        let stored = HashMap::from([
            ("notification_seconds", "0".to_string()),
            ("notification_duration", "forever".to_string()),
            (
                "notification_custom_icon",
                "https://example.com/a.png".to_string(),
            ),
        ]);
        let read = AppSettings::from_stored(|name| stored.get(name).cloned());

        assert_eq!(read, AppSettings::default());
    }

    #[test]
    fn checks_what_no_notification_could_show() {
        assert_eq!(AppSettings::default().check(), Ok(()));

        let mut long = AppSettings::default();
        long.notifications.body = "x".repeat(TEMPLATE_LIMIT + 1);
        assert_eq!(long.check().unwrap_err().kind, ErrorKind::Invalid);

        let mut never = AppSettings::default();
        never.notifications.seconds = 0;
        assert!(never.check().is_err());

        let mut remote = AppSettings::default();
        remote.notifications.custom_icon = Some("https://example.com/a.png".to_string());
        assert!(remote.check().is_err());
    }

    #[test]
    fn the_message_is_downloaded_only_when_a_template_shows_it() {
        let mut settings = NotificationSettings::default();
        assert!(settings.shows_preview());

        settings.body = "{subject}".to_string();
        assert!(!settings.shows_preview());

        settings.title = "{preview}".to_string();
        assert!(settings.shows_preview());

        settings.enabled = false;
        assert!(!settings.shows_preview());
    }
}
