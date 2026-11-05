use crownui::kit::{icon, use_theme};
use crownui::prelude::{Cx, IntoProp, Styled, View, text, vstack};

use crate::util::tint::tint;

const GLYPH_SIZE: f32 = 48.0;
const BODY_WIDTH: f32 = 360.0;

/// A glyph, a title and a line of explanation, stacked and centered.
pub fn empty_state<Body: IntoProp<String> + 'static>(
    cx: &mut Cx,
    glyph: &'static str,
    title: &'static str,
    body: Body,
) -> impl View + use<Body> {
    let theme = use_theme(cx);
    vstack((
        icon(glyph).size(GLYPH_SIZE),
        text(title)
            .font_semibold()
            .text_size(18.0)
            .text_color(tint(theme, |theme| theme.text.primary)),
        text(body)
            .text_center()
            .max_w(BODY_WIDTH)
            .text_color(tint(theme, |theme| theme.text.muted)),
    ))
    .gap(10.0)
    .items_center()
}
