//! Per-instance presentation identity, independent of persisted layout and engine authority.
//!
//! The host selects it before the first frame (the web build: the exact `embedded` query key) and
//! it never changes afterwards. It is not stored in UiState, preferences, the catalog or a project,
//! and it grants no authority: it only removes the upstream community routes and product naming
//! from what the LTS editor shows, lists and opens. Command ids, shortcut tables and formats are
//! the same in both presentations.

/// The plain-text upstream attribution (About and Third-party notices, in both presentations).
pub const ATTRIBUTION: &str = "Modified version based on LightCraft by the ArtCraft team and community.";
/// This engine's full name in the LTS editor (About heading; the page uses it too).
pub const EMBEDDED_NAME: &str = "LTS Image adjustments";
/// Refusal for the five community and upstream-support commands and their destinations.
pub const COMMUNITY_REFUSAL: &str = "Community navigation is unavailable in the LTS editor";
/// Refusal for any other external page in the LTS editor.
pub const EXTERNAL_REFUSAL: &str = "External links are unavailable in the LTS editor";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Presentation {
    #[default]
    Standalone,
    LtsEmbedded,
}

impl Presentation {
    /// Upstream community routes, product naming and external links: the standalone app.
    pub fn shows_community(self) -> bool {
        self == Self::Standalone
    }

    /// Whether the command runs, and is listed in menus, command catalogs and the shortcuts list.
    /// The LTS editor refuses the five community commands; `app.help` stays (there it opens the
    /// local Keyboard Shortcuts).
    pub fn allows_command(self, id: &str) -> bool {
        self.shows_community() || id == "app.help" || crate::links::url_of(id).is_none()
    }

    /// Why `url` may not be opened (`None`: it may). The LTS editor opens no external page at all:
    /// the community destinations get the community message, anything else the external one.
    pub fn url_refusal(self, url: &str) -> Option<&'static str> {
        if self.shows_community() {
            None
        } else if crate::links::LINKS.iter().any(|(id, _, destination)| *id != "app.help" && *destination == url) {
            Some(COMMUNITY_REFUSAL)
        } else {
            Some(EXTERNAL_REFUSAL)
        }
    }

    /// A command's label as this presentation shows it (menus, catalogs, shortcuts list, About
    /// title): the LTS editor relabels the product-named ones. Ids never change.
    pub fn command_label<'a>(self, id: &str, original: &'a str) -> &'a str {
        match (self, id) {
            (Self::LtsEmbedded, "app.help") => "Editing shortcuts",
            (Self::LtsEmbedded, "app.about") => "Editing engine information",
            (Self::LtsEmbedded, "app.quit") => "Quit editor",
            _ => original,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_refuses_exactly_the_five_community_commands() {
        for &(id, _, _) in crate::links::LINKS {
            assert_eq!(Presentation::LtsEmbedded.allows_command(id), id == "app.help", "{id}");
            assert!(Presentation::Standalone.allows_command(id), "{id}");
        }
        for id in ["app.about", "app.quit", "app.whatsNew", "app.shortcuts", "app.systemInfo", "view.detail", "develop.set"] {
            assert!(Presentation::LtsEmbedded.allows_command(id), "{id}");
        }
    }

    #[test]
    fn embedded_refuses_every_url_with_the_matching_message() {
        for &(id, _, url) in crate::links::LINKS {
            let refusal = if id == "app.help" { EXTERNAL_REFUSAL } else { COMMUNITY_REFUSAL };
            assert_eq!(Presentation::LtsEmbedded.url_refusal(url), Some(refusal), "{id}");
            assert_eq!(Presentation::Standalone.url_refusal(url), None, "{id}");
        }
        for url in ["https://www.openstreetmap.org/?mlat=1&mlon=2", "https://github.com/some-contributor", ""] {
            assert_eq!(Presentation::LtsEmbedded.url_refusal(url), Some(EXTERNAL_REFUSAL), "{url}");
            assert_eq!(Presentation::Standalone.url_refusal(url), None, "{url}");
        }
    }

    /// Only the product-named labels change, and nothing the LTS editor lists names the upstream product.
    #[test]
    fn only_product_named_labels_change() {
        for &(id, label, _, _) in crate::menus::ui_commands() {
            let embedded = Presentation::LtsEmbedded.command_label(id, label);
            assert_eq!(Presentation::Standalone.command_label(id, label), label, "{id}");
            if !matches!(id, "app.help" | "app.about" | "app.quit") {
                assert_eq!(embedded, label, "{id}");
            }
            if Presentation::LtsEmbedded.allows_command(id) {
                for upstream in ["LightCraft", "ArtCraft", "Discord", "GitHub"] {
                    assert!(!embedded.contains(upstream), "{id}: {embedded}");
                }
            }
        }
    }
}
