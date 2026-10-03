use egui::Color32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    pub fn toggle(&self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        }
    }

    pub fn button_label(&self) -> &'static str {
        match self {
            ThemeMode::Light => "☾ Dark",
            ThemeMode::Dark => "☼ Light",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub mode: ThemeMode,
    pub bg_canvas: Color32,
    pub bg_panel: Color32,
    pub bg_selected: Color32,
    pub bg_code: Color32,
    pub bg_card: Color32,
    #[allow(dead_code)]
    pub bg_card_hover: Color32,
    pub border_subtle: Color32,
    pub border_card: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub text_gutter: Color32,
    pub accent_blue: Color32,
    pub accent_green: Color32,
    pub diff_add_bg: Color32,
    pub diff_add_text: Color32,
    pub diff_del_bg: Color32,
    pub diff_del_text: Color32,
    pub diff_hdr_bg: Color32,
    pub diff_hdr_text: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            bg_canvas: Color32::from_rgb(13, 17, 23),       // #0D1117 (Deep obsidian base)
            bg_panel: Color32::from_rgb(22, 27, 34),        // #161B22 (Panel surface)
            bg_selected: Color32::from_rgb(38, 56, 82),     // #263852 (High-contrast selection)
            bg_code: Color32::from_rgb(16, 20, 26),         // #10141A (Diff code canvas)
            bg_card: Color32::from_rgb(28, 35, 45),         // #1C232D (Elevated card, clearly distinct from canvas)
            bg_card_hover: Color32::from_rgb(36, 46, 60),   // #242E3C (Interactive hover)
            border_subtle: Color32::from_rgb(52, 60, 72),   // #343C48 (Hairline dividers, clearly visible)
            border_card: Color32::from_rgb(64, 74, 90),     // #404A5A (Crisp card boundaries)
            text_primary: Color32::from_rgb(240, 246, 252), // #F0F6FC (Sharp white typography)
            text_muted: Color32::from_rgb(160, 172, 186),   // #A0ACBA (Readable secondary labels)
            text_gutter: Color32::from_rgb(125, 137, 152),  // #7D8998 (Legible line numbers)
            accent_blue: Color32::from_rgb(88, 166, 255),   // #58A6FF (Tactical vibrant blue)
            accent_green: Color32::from_rgb(63, 185, 80),   // #3FB950 (Vibrant indicator green)
            diff_add_bg: Color32::from_rgba_premultiplied(46, 160, 67, 45),
            diff_add_text: Color32::from_rgb(86, 211, 100),  // #56D364
            diff_del_bg: Color32::from_rgba_premultiplied(248, 81, 73, 45),
            diff_del_text: Color32::from_rgb(248, 81, 73),   // #F85149
            diff_hdr_bg: Color32::from_rgba_premultiplied(56, 139, 253, 35),
            diff_hdr_text: Color32::from_rgb(121, 192, 255), // #79C0FF
        }
    }

    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            bg_canvas: Color32::from_rgb(246, 248, 250),     // #F6F8FA (GitHub light canvas)
            bg_panel: Color32::from_rgb(255, 255, 255),      // #FFFFFF (Surface white)
            bg_selected: Color32::from_rgb(222, 235, 255),   // #DEEBFF (Subtle blue-gray tint)
            bg_code: Color32::from_rgb(246, 248, 250),       // #F6F8FA (Code block canvas)
            bg_card: Color32::from_rgb(255, 255, 255),       // #FFFFFF (Card background)
            bg_card_hover: Color32::from_rgb(243, 245, 248), // #F3F5F8 (Card hover)
            border_subtle: Color32::from_rgb(216, 222, 228), // #D8DEE4 (Dividers)
            border_card: Color32::from_rgb(208, 215, 222),   // #D0D7DE (Card boundary)
            text_primary: Color32::from_rgb(31, 35, 40),     // #1F2328 (Deep graphite)
            text_muted: Color32::from_rgb(101, 109, 118),    // #656D76 (Secondary text)
            text_gutter: Color32::from_rgb(140, 149, 159),   // #8C959F (Gutter numbers)
            accent_blue: Color32::from_rgb(9, 105, 218),     // #0969DA (Action Blue)
            accent_green: Color32::from_rgb(26, 127, 55),    // #1A7F37 (Indicator Green)
            diff_add_bg: Color32::from_rgba_premultiplied(46, 160, 67, 35),
            diff_add_text: Color32::from_rgb(26, 127, 55),   // #1A7F37 (High contrast green)
            diff_del_bg: Color32::from_rgba_premultiplied(207, 34, 46, 35),
            diff_del_text: Color32::from_rgb(207, 34, 46),   // #CF222E (High contrast red)
            diff_hdr_bg: Color32::from_rgba_premultiplied(9, 105, 218, 25),
            diff_hdr_text: Color32::from_rgb(9, 105, 218),   // #0969DA
        }
    }

    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
        }
    }
}
