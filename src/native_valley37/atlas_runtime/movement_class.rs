//! Original 004A8AD0 cliff flag quartet on the nonnegative authored grid.

pub fn half_cell_obstructed([x, y]: [i32; 2], tile_flags: Option<u16>) -> bool {
    let x_mask = if x & 1 == 0 { 0xf000 } else { 0x00f0 };
    let y_mask = if y & 1 == 0 { 0x000f } else { 0x0f00 };
    tile_flags.is_some_and(|flags| flags & (x_mask | y_mask) != 0)
}
