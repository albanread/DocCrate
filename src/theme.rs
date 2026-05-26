// VS Code Dark+ inspired palette

use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;

pub fn hex(v: u32) -> D2D1_COLOR_F {
    let r = ((v >> 16) & 0xFF) as f32 / 255.0;
    let g = ((v >> 8) & 0xFF) as f32 / 255.0;
    let b = (v & 0xFF) as f32 / 255.0;
    D2D1_COLOR_F { r, g, b, a: 1.0 }
}

pub const BG:           u32 = 0x1E1E1E;
pub const SIDEBAR_BG:   u32 = 0x252526;
pub const SIDEBAR_SEL:  u32 = 0x37373D;
pub const SIDEBAR_HVR:  u32 = 0x2A2D2E;
pub const BORDER:       u32 = 0x3C3C3C;
pub const SCROLLBAR:    u32 = 0x424242;
pub const SCROLLTHUMB:  u32 = 0x686868;

pub const TEXT:         u32 = 0xD4D4D4;
pub const TEXT_DIM:     u32 = 0x808080;
pub const TEXT_BRIGHT:  u32 = 0xFFFFFF;

pub const H1:           u32 = 0x4EC9B0; // teal
pub const H2:           u32 = 0x9CDCFE; // light blue
pub const H3:           u32 = 0xDCDCAA; // yellow
pub const H4:           u32 = 0xC586C0; // purple
pub const H5:           u32 = 0xCE9178; // orange
pub const H6:           u32 = 0x808080; // dim

pub const LINK:         u32 = 0x4FC1FF;
pub const LINK_HVR:     u32 = 0x87D7FF;
pub const CODE_FG:      u32 = 0xCE9178;
pub const CODE_BG:      u32 = 0x0D0D0D;
pub const BLOCKQUOTE:   u32 = 0x608B4E;
pub const RULE:         u32 = 0x3C3C3C;

pub const BODY_FONT:    &str = "Segoe UI";
pub const CODE_FONT:    &str = "Cascadia Code";
pub const CODE_FONT2:   &str = "Consolas"; // fallback

pub const BODY_SIZE:    f32 = 15.0;
pub const CODE_SIZE:    f32 = 13.5;
pub const H1_SIZE:      f32 = 30.0;
pub const H2_SIZE:      f32 = 24.0;
pub const H3_SIZE:      f32 = 20.0;
pub const H4_SIZE:      f32 = 17.0;
pub const H5_SIZE:      f32 = BODY_SIZE;
pub const H6_SIZE:      f32 = BODY_SIZE;

pub const SIDEBAR_W:    f32 = 220.0;
pub const SCROLLBAR_W:  f32 = 10.0;
pub const H_PAD:        f32 = 32.0;
pub const V_PAD:        f32 = 20.0;
pub const PARA_GAP:     f32 = 10.0;
pub const LINE_EXTRA:   f32 = 1.4; // line-height multiplier
pub const H_RULE_H:     f32 = 1.0;
pub const CODE_PAD:     f32 = 12.0;
pub const BQ_BAR_W:     f32 = 4.0;
pub const BQ_PAD:       f32 = 16.0;
pub const SIDEBAR_ITEM_H: f32 = 26.0;
pub const SIDEBAR_FONT_SIZE: f32 = 13.0;
