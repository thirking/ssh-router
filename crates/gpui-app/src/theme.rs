use gpui::{hsla, px, Hsla, Pixels};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppTheme {
    pub background: Hsla,
    pub surface: Hsla,
    pub surface_hover: Hsla,
    pub border: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub primary: Hsla,
    pub primary_hover: Hsla,
    pub danger: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub focus: Hsla,
    pub overlay: Hsla,
    pub font_body: Pixels,
    pub font_small: Pixels,
    pub space_sm: Pixels,
    pub space_md: Pixels,
    pub border_width: Pixels,
    pub radius: Pixels,
    pub disabled_opacity: f32,
    pub active_opacity: f32,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            background: hsla(222.2 / 360.0, 0.47, 0.11, 1.0),
            surface: hsla(217.2 / 360.0, 0.33, 0.17, 1.0),
            surface_hover: hsla(215.3 / 360.0, 0.25, 0.27, 1.0),
            border: hsla(215.3 / 360.0, 0.25, 0.27, 1.0),
            text: hsla(210.0 / 360.0, 0.40, 0.98, 1.0),
            text_muted: hsla(215.4 / 360.0, 0.16, 0.65, 1.0),
            primary: hsla(217.2 / 360.0, 0.91, 0.60, 1.0),
            primary_hover: hsla(221.2 / 360.0, 0.83, 0.53, 1.0),
            danger: hsla(0.0, 0.84, 0.60, 1.0),
            success: hsla(142.1 / 360.0, 0.71, 0.45, 1.0),
            warning: hsla(37.7 / 360.0, 0.92, 0.50, 1.0),
            focus: hsla(213.1 / 360.0, 0.94, 0.68, 1.0),
            overlay: hsla(222.2 / 360.0, 0.47, 0.05, 0.72),
            font_body: px(14.0),
            font_small: px(12.0),
            space_sm: px(8.0),
            space_md: px(16.0),
            border_width: px(1.0),
            radius: px(6.0),
            disabled_opacity: 0.45,
            active_opacity: 0.8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_colors_are_opaque_except_overlay() {
        let theme = AppTheme::default();

        assert_eq!(theme.background.a, 1.0);
        assert_eq!(theme.primary.a, 1.0);
        assert!(theme.overlay.a < 1.0);
    }
}
