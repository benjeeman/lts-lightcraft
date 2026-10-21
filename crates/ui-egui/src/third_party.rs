//! About ▸ Third-party notices: the licence and attribution texts this build carries, verbatim
//! from the repository, as selectable plain text with no links (in both presentations). It does
//! not claim to be complete; [`SCOPE`] says what is missing.

use crate::theme::Tokens;

/// What the view does not cover, shown under the attribution.
pub const SCOPE: &str = "These notices are not a complete list: the licences of the Rust dependencies compiled into this \
    program are governed by Cargo.lock, and a generated dependency-licence bundle is pending.";

/// (title, repository path, text) in display order, each included verbatim at build time.
pub const SECTIONS: &[(&str, &str, &str)] = &[
    ("Notice", "NOTICE", include_str!("../../../NOTICE")),
    ("Asset attribution", "assets/ATTRIBUTION.md", include_str!("../../../assets/ATTRIBUTION.md")),
    ("Inter font licence", "assets/fonts/OFL-Inter.txt", include_str!("../../../assets/fonts/OFL-Inter.txt")),
    ("App icon licence", "assets/app-icon/LICENSE.txt", include_str!("../../../assets/app-icon/LICENSE.txt")),
    ("egui default fonts", "docs/notices/egui-default-fonts.txt", include_str!("../../../docs/notices/egui-default-fonts.txt")),
    ("MIT License", "LICENSE-MIT", include_str!("../../../LICENSE-MIT")),
    ("Apache License 2.0", "LICENSE-APACHE", include_str!("../../../LICENSE-APACHE")),
];

/// The scroll area's height: the About dialog is a fixed-size, centred window.
const HEIGHT: f32 = 360.0;

/// The view's blocks, built once: (is a heading, text). Texts are split at blank lines, so only
/// what is scrolled into view is painted, and their line endings are `\n` (a checkout may have
/// `\r\n`).
fn blocks() -> &'static [(bool, String)] {
    static BLOCKS: std::sync::OnceLock<Vec<(bool, String)>> = std::sync::OnceLock::new();
    BLOCKS.get_or_init(|| {
        let mut out = vec![(false, crate::presentation::ATTRIBUTION.to_string()), (false, SCOPE.to_string())];
        for (title, path, text) in SECTIONS {
            out.push((true, format!("{title} ({path})")));
            let text = text.replace("\r\n", "\n");
            out.extend(text.split("\n\n").map(|p| p.trim_matches('\n')).filter(|p| !p.trim().is_empty()).map(|p| (false, p.to_string())));
        }
        out
    })
}

/// The whole view as plain text (what selecting all of it copies).
pub fn text() -> String {
    blocks().iter().map(|(_, block)| block.as_str()).collect::<Vec<_>>().join("\n\n")
}

pub fn ui(ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::ScrollArea::vertical().id_salt("third_party_scroll").max_height(HEIGHT).auto_shrink([false, true]).show(ui, |ui| {
        for (heading, block) in blocks() {
            if *heading {
                ui.add_space(8.0);
                ui.add(egui::Label::new(egui::RichText::new(block).font(t.semibold(12.5)).color(t.text)).selectable(true));
            } else {
                ui.add(egui::Label::new(egui::RichText::new(block).monospace().size(11.0).color(t.text_label)).selectable(true).wrap());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every included text appears whole (paragraph by paragraph, line endings aside), after the
    /// attribution and the scope: NOTICE with its SAM 3 paragraph, the licences, the egui fonts.
    #[test]
    fn notices_carry_the_repository_texts_verbatim() {
        let all = text();
        assert!(all.starts_with(crate::presentation::ATTRIBUTION), "{all}");
        assert!(all.contains("not a complete list") && all.contains("Cargo.lock"));
        assert!(!all.contains('\r'));
        for (_, path, body) in SECTIONS {
            let body = body.replace("\r\n", "\n");
            assert!(!body.trim().is_empty(), "{path} is empty");
            for paragraph in body.split("\n\n").map(|p| p.trim_matches('\n')).filter(|p| !p.trim().is_empty()) {
                assert!(all.contains(paragraph), "{path}: {paragraph}");
            }
        }
        for required in [
            "Copyright (c) 2026 ArtCraft Team and the LightCraft contributors.",
            "crates/segment (lightcraft-segment) ports the SAM 3 model code of Hugging Face Transformers",
            "Copyright The HuggingFace Team and Meta Platforms, Inc., licensed under the Apache License 2.0.",
            "The SAM 3 weights are not part of LightCraft and are never distributed with it.",
            "SIL OPEN FONT LICENSE Version 1.1",
            "UBUNTU FONT LICENCE Version 1.0",
            "BITSTREAM VERA LICENSE",
            "Copyright (c) 2014 John Slegers",
            "Copyright 2013 Google Inc.",
            "Apache License",
            "MIT License",
        ] {
            assert!(all.contains(required), "{required}");
        }
    }
}
