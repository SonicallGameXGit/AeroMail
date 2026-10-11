//! Which tab of the inbox a message is shown in: Primary, Social,
//! Promotions, Updates or Forums, as in Gmail. Gmail decides with a model
//! trained on everyone's mail; here it is plain rules over what the
//! message's header says, so the result can always be predicted:
//!
//! 1. sent by a social network (`SOCIAL`): Social;
//! 2. sent through a discussion list one can write to (`List-Post`), or
//!    by a group host (`GROUPS`): Forums;
//! 3. a subject that reads like a receipt, a delivery, a code or a
//!    statement (`TRANSACTIONAL`): Updates;
//! 4. sent by a marketing platform's campaign, marked as bulk mail, or
//!    from a marketing address (`news@`, `offers@`, ...): Promotions;
//! 5. sent by a machine (`Auto-Submitted`, `noreply@`, `alerts@`, ...):
//!    Updates;
//! 6. mass mail one can leave (`List-Unsubscribe`, `List-Id`): Promotions;
//! 7. everything else: Primary.
//!
//! On top of that, what the user decided wins (`decide`): a sender they
//! put in a category stays there, and mail from people they wrote to stays
//! in Primary. A category that is turned off sends its mail to Primary.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::settings::CategorySettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Category {
    /// Mail from people, and anything no rule recognises.
    Primary,
    /// Social networks and media-sharing sites.
    Social,
    /// Offers, newsletters and other marketing.
    Promotions,
    /// Receipts, deliveries, codes, statements, notifications.
    Updates,
    /// Mailing lists and discussion groups.
    Forums,
}

impl Category {
    /// Every category, in the order the tabs show them.
    pub const ALL: [Category; 5] = [
        Category::Primary,
        Category::Social,
        Category::Promotions,
        Category::Updates,
        Category::Forums,
    ];

    /// The name it is stored by.
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Primary => "primary",
            Category::Social => "social",
            Category::Promotions => "promotions",
            Category::Updates => "updates",
            Category::Forums => "forums",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|category| category.as_str() == text)
    }
}

/// "Mail from this sender goes in this category", decided by the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CategoryRule {
    /// The sender's address, in small letters.
    pub sender: String,
    pub category: Category,
}

/// What a message's header says about where it comes from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Signals<'a> {
    /// The sender's address.
    pub sender: &'a str,
    pub subject: &'a str,
    /// `List-Id`: sent through a list.
    pub list_id: bool,
    /// `List-Post` with an address: a list its readers write to.
    pub list_post: bool,
    pub list_unsubscribe: bool,
    /// `Precedence: bulk` (or `junk`).
    pub bulk: bool,
    /// `Auto-Submitted` other than `no`: written by a machine.
    pub auto_submitted: bool,
    /// A header marketing platforms put on their campaigns.
    pub campaign: bool,
}

/// Social networks and media-sharing sites, with the domains they send
/// from. A sender's domain matches itself and its subdomains.
const SOCIAL: [&str; 29] = [
    "facebook.com",
    "facebookmail.com",
    "instagram.com",
    "threads.net",
    "whatsapp.com",
    "linkedin.com",
    "twitter.com",
    "x.com",
    "tiktok.com",
    "snapchat.com",
    "pinterest.com",
    "reddit.com",
    "redditmail.com",
    "tumblr.com",
    "quora.com",
    "discord.com",
    "vk.com",
    "vk.ru",
    "ok.ru",
    "youtube.com",
    "twitch.tv",
    "flickr.com",
    "deviantart.com",
    "soundcloud.com",
    "meetup.com",
    "nextdoor.com",
    "strava.com",
    "goodreads.com",
    "telegram.org",
];

/// Hosts of discussion groups, some of which leave `List-Post` out.
const GROUPS: [&str; 4] = [
    "googlegroups.com",
    "groups.io",
    "discoursemail.com",
    "freelists.org",
];

/// Addresses marketing is sent from: the part before the @.
const MARKETING_SENDERS: [&str; 10] = [
    "news",
    "newsletter",
    "newsletters",
    "marketing",
    "offers",
    "deals",
    "promo",
    "promotions",
    "sales",
    "specials",
];

/// Addresses machines send from: the part before the @.
const MACHINE_SENDERS: [&str; 22] = [
    "noreply",
    "no-reply",
    "no_reply",
    "donotreply",
    "do-not-reply",
    "do_not_reply",
    "notification",
    "notifications",
    "notify",
    "alert",
    "alerts",
    "security",
    "account",
    "accounts",
    "billing",
    "invoice",
    "invoices",
    "receipts",
    "orders",
    "mailer-daemon",
    "postmaster",
    "automated",
];

/// Words that start a receipt, a delivery, a code or a statement, in the
/// languages mail is commonly written in. Each matches at the start of a
/// word of the subject, in small letters, punctuation taken out and "ё"
/// read as "е", so "Receipt", "чека" and "Rechnungen" all count.
const TRANSACTIONAL: [&str; 78] = [
    // English
    "receipt",
    "invoice",
    "order confirmation",
    "order number",
    "has shipped",
    "has been shipped",
    "out for delivery",
    "has been delivered",
    "verification code",
    "security code",
    "confirmation code",
    "one time code",
    "one time password",
    "password",
    "new sign in",
    "new login",
    "security alert",
    "statement",
    "payment received",
    "payment confirmation",
    "booking confirmation",
    "reservation confirmation",
    "itinerary",
    "your bill",
    // Russian
    "чек",
    "квитанц",
    "счет на оплату",
    "код подтверждения",
    "одноразовый код",
    "пароль",
    "вход в аккаунт",
    "новый вход",
    "ваш заказ",
    "заказ оформлен",
    "заказ отправлен",
    "заказ доставлен",
    "номер заказа",
    "выписка",
    "оплата получена",
    "электронный билет",
    // Ukrainian
    "рахунок",
    "код підтвердження",
    "ваше замовлення",
    "замовлення оформлено",
    "виписка",
    "оплату отримано",
    // Polish
    "faktura",
    "paragon",
    "potwierdzenie zamówienia",
    "twoje zamówienie",
    "zamówienie nr",
    "kod weryfikacyjny",
    "hasło",
    "wyciąg",
    "potwierdzenie płatności",
    // German
    "rechnung",
    "quittung",
    "bestellbestätigung",
    "ihre bestellung",
    "versandbestätigung",
    "bestätigungscode",
    "passwort",
    "kontoauszug",
    "zahlungsbestätigung",
    "buchungsbestätigung",
    // French
    "facture",
    "reçu",
    "confirmation de commande",
    "votre commande",
    "code de vérification",
    "mot de passe",
    "relevé",
    // Spanish
    "factura",
    "recibo",
    "confirmación de pedido",
    "tu pedido",
    "código de verificación",
    "contraseña",
];

/// Whether `domain` is one of `list` or below one of them.
fn sent_by(domain: &str, list: &[&str]) -> bool {
    list.iter().any(|known| {
        domain == *known
            || domain
                .strip_suffix(known)
                .is_some_and(|rest| rest.ends_with('.'))
    })
}

/// The subject as words in small letters, one space between them and one
/// in front, so a word's start is always " word".
fn words(subject: &str) -> String {
    let mut words = String::from(" ");
    for word in subject
        .to_lowercase()
        .replace('ё', "е")
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
    {
        words.push_str(word);
        words.push(' ');
    }

    words
}

fn reads_like_a_transaction(subject: &str) -> bool {
    let words = words(subject);

    TRANSACTIONAL
        .iter()
        .any(|start| words.contains(&format!(" {start}")))
}

/// The category the header alone points to (the rules at the top).
pub fn classify(signals: &Signals<'_>) -> Category {
    let sender = signals.sender.trim().to_lowercase();
    let (local, domain) = sender.rsplit_once('@').unwrap_or(("", ""));
    let local = local.split('+').next().unwrap_or(local);

    if sent_by(domain, &SOCIAL) {
        return Category::Social;
    }
    if signals.list_post || sent_by(domain, &GROUPS) {
        return Category::Forums;
    }
    if reads_like_a_transaction(signals.subject) {
        return Category::Updates;
    }
    if signals.campaign || signals.bulk || MARKETING_SENDERS.contains(&local) {
        return Category::Promotions;
    }
    if signals.auto_submitted || MACHINE_SENDERS.contains(&local) {
        return Category::Updates;
    }
    if signals.list_unsubscribe || signals.list_id {
        return Category::Promotions;
    }

    Category::Primary
}

/// The tab a message is shown in. The user's own rule for its sender comes
/// first, then mail from someone the user wrote to (`correspondent`) stays
/// in Primary, then the header decides (`automatic`, from `classify`).
pub fn decide(
    automatic: Category,
    rule: Option<Category>,
    correspondent: bool,
    settings: &CategorySettings,
) -> Category {
    let category = match rule {
        Some(rule) => rule,
        None if correspondent => Category::Primary,
        None => automatic,
    };

    if settings.shows(category) {
        category
    } else {
        Category::Primary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from(sender: &'static str, subject: &'static str) -> Signals<'static> {
        Signals {
            sender,
            subject,
            ..Signals::default()
        }
    }

    #[test]
    fn names_round_trip() {
        for category in Category::ALL {
            assert_eq!(Category::parse(category.as_str()), Some(category));
        }
        assert_eq!(Category::parse("Social"), None);
    }

    #[test]
    fn mail_from_a_person_is_primary() {
        assert_eq!(
            classify(&from("ann@example.com", "Lunch on Friday?")),
            Category::Primary
        );
        assert_eq!(classify(&Signals::default()), Category::Primary);
    }

    #[test]
    fn social_networks_by_their_domain_and_subdomains() {
        let cases = [
            "notification@facebookmail.com",
            "no-reply@mail.instagram.com",
            "messages-noreply@linkedin.com",
            "noreply@redditmail.com",
        ];
        for sender in cases {
            let signals = Signals {
                list_unsubscribe: true,
                ..from(sender, "You have a new message")
            };
            assert_eq!(classify(&signals), Category::Social, "{sender}");
        }
        // Only a whole part of the domain matches.
        assert_eq!(
            classify(&from("ann@notfacebook.com", "Hi")),
            Category::Primary
        );
        assert_eq!(classify(&from("ann@box.com", "Hi")), Category::Primary);
    }

    #[test]
    fn a_list_its_readers_write_to_is_a_forum() {
        let list = Signals {
            list_id: true,
            list_post: true,
            list_unsubscribe: true,
            ..from("ann@example.com", "[rust-users] Borrowing question")
        };
        assert_eq!(classify(&list), Category::Forums);
        assert_eq!(
            classify(&from("someone@googlegroups.com", "Meeting")),
            Category::Forums
        );
    }

    #[test]
    fn receipts_codes_and_deliveries_are_updates_in_many_languages() {
        let subjects = [
            "Your receipt from Example Shop",
            "Your order has shipped",
            "Your verification code is 123456",
            "Security alert: new sign-in on Linux",
            "Ваш чек от магазина",
            "Код подтверждения: 4821",
            "Ваш заказ отправлен",
            "Twoje zamówienie zostało wysłane",
            "Ihre Rechnung für Oktober",
            "Votre commande est confirmée",
            "Tu pedido ha sido enviado",
        ];
        for subject in subjects {
            let signals = Signals {
                list_unsubscribe: true,
                ..from("shop@example.com", subject)
            };
            assert_eq!(classify(&signals), Category::Updates, "{subject}");
        }
    }

    #[test]
    fn a_word_counts_only_from_its_start() {
        // "Расчёт" holds "счёт", "misstatement" holds "statement".
        assert_eq!(
            classify(&from("ann@example.com", "Расчёт на оплату")),
            Category::Primary
        );
        assert_eq!(
            classify(&from("ann@example.com", "A misstatement")),
            Category::Primary
        );
    }

    #[test]
    fn campaigns_bulk_mail_and_marketing_addresses_are_promotions() {
        let campaign = Signals {
            campaign: true,
            ..from("hello@shop.example", "Autumn sale: 30% off")
        };
        let bulk = Signals {
            bulk: true,
            ..from("hello@shop.example", "New arrivals")
        };
        assert_eq!(classify(&campaign), Category::Promotions);
        assert_eq!(classify(&bulk), Category::Promotions);
        assert_eq!(
            classify(&from("Newsletter+fr@shop.example", "This week")),
            Category::Promotions
        );
    }

    #[test]
    fn machines_write_updates() {
        let automated = Signals {
            auto_submitted: true,
            ..from("robot@ci.example", "Build finished")
        };
        assert_eq!(classify(&automated), Category::Updates);
        // A notifier with a way to unsubscribe is still a notifier.
        let notifier = Signals {
            list_id: true,
            list_unsubscribe: true,
            ..from(
                "notifications@github.com",
                "[owner/repo] Fix the build (#12)",
            )
        };
        assert_eq!(classify(&notifier), Category::Updates);
    }

    #[test]
    fn other_mass_mail_one_can_leave_is_a_promotion() {
        let newsletter = Signals {
            list_unsubscribe: true,
            ..from("hello@startup.example", "What we built in October")
        };
        assert_eq!(classify(&newsletter), Category::Promotions);
    }

    #[test]
    fn the_users_rule_wins_then_people_they_wrote_to_then_the_header() {
        let all = CategorySettings {
            enabled: true,
            social: true,
            promotions: true,
            updates: true,
            forums: true,
        };
        let promotion = Category::Promotions;

        assert_eq!(decide(promotion, None, false, &all), Category::Promotions);
        assert_eq!(decide(promotion, None, true, &all), Category::Primary);
        assert_eq!(
            decide(promotion, Some(Category::Updates), true, &all),
            Category::Updates
        );
        assert_eq!(
            decide(Category::Primary, Some(Category::Social), false, &all),
            Category::Social
        );
    }

    #[test]
    fn a_category_turned_off_sends_its_mail_to_primary() {
        let some = CategorySettings {
            enabled: true,
            social: true,
            promotions: false,
            updates: true,
            forums: false,
        };

        assert_eq!(
            decide(Category::Promotions, None, false, &some),
            Category::Primary
        );
        assert_eq!(
            decide(Category::Updates, Some(Category::Forums), false, &some),
            Category::Primary
        );
        assert_eq!(
            decide(Category::Social, None, false, &some),
            Category::Social
        );
    }
}
