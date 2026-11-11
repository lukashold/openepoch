//! Original 004A8A30 and ordinary 004AA455..004AA750 branch operations.

pub const DEFAULT_DENSITY: f32 = 6.0;
pub const MIXED: u8 = 0x3f;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Square {
    pub level: u8,
    pub x: u16,
    pub y: u16,
}

impl Square {
    pub fn children(self) -> Option<[Self; 4]> {
        let level = self.level.checked_sub(1)?;
        let side = 1 << level;
        Some(std::array::from_fn(|i| Self {
            level,
            x: self.x + (i & 1) as u16 * side,
            y: self.y + (i >> 1) as u16 * side,
        }))
    }

    pub fn center(self) -> [f32; 2] {
        let offset = f32::from(1_u16 << self.level) * 0.25;
        [self.x, self.y].map(|axis| f32::from(axis) * 0.5 + offset)
    }

    pub fn alias_slots(
        self,
        slot_level: u8,
        width: usize,
        height: usize,
        branched: bool,
    ) -> impl Iterator<Item = usize> {
        let [x, y] = [self.x, self.y].map(|axis| usize::from(axis >> slot_level));
        let span = if self.level < slot_level || branched && self.level != slot_level {
            0
        } else {
            1_usize << (self.level - slot_level)
        };
        (y..(y + span).min(height))
            .flat_map(move |row| (x..(x + span).min(width)).map(move |col| row * width + col))
    }
}

#[derive(Clone, Copy)]
pub struct Child {
    pub class: u8,
    pub density: f32,
    pub anchor: Option<u32>,
}

pub enum Collapse {
    RetainBranch,
    Leaf { class: u8, density: f32 },
}

pub fn collapse(level: u8, children: [Child; 4]) -> Collapse {
    let class = children[0].class;
    if class == MIXED
        || children
            .iter()
            .any(|child| child.class != class || child.anchor.is_some())
    {
        return Collapse::RetainBranch;
    }
    let density = children
        .iter()
        .fold(DEFAULT_DENSITY, |min, child| min.min(child.density));
    if density > 0.0 && density < f32::from(level.min(4)) {
        return Collapse::RetainBranch;
    }
    let class = match class {
        7 if density < 1.59_f32 => 5,
        5 if density >= 1.59_f32 => 7,
        class => class,
    };
    Collapse::Leaf { class, density }
}
