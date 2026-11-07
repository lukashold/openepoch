//! Current authenticated Valley LChunkFile → Wrld / Wrld / UtFt.
use super::source_forest_grid::SourceForestGrid;

pub(super) const NO_LINK: usize = u32::MAX as usize;
const HEADER: usize = 20;
const TREE_EXTENT: f32 = f32::from_bits(1061625612);

pub(super) struct Chunk<'a> {
    pub(super) tag: &'a [u8],
    pub(super) payload: usize,
    pub(super) end: usize,
    pub(super) sibling: usize,
    pub(super) child: usize,
}

fn word(data: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(data.get(at..at.checked_add(4)?)?.try_into().ok()?) as usize)
}

impl<'a> Chunk<'a> {
    pub(super) fn read(data: &'a [u8], at: usize, limit: usize) -> Option<Self> {
        let payload = at.checked_add(HEADER)?;
        let header = data.get(at..payload)?;
        let end = word(header, 4)?;
        let sibling = word(header, 12)?;
        let child = word(header, 16)?;
        if word(header, 8)? != 1
            || payload > end
            || end > limit
            || (sibling != NO_LINK && !(end..limit).contains(&sibling))
            || (child != NO_LINK && !(payload..end).contains(&child))
        {
            return None;
        }
        Some(Self {
            tag: &header[..4],
            payload,
            end,
            sibling,
            child,
        })
    }

    pub(super) fn find_unique(&self, data: &'a [u8], tag: &[u8; 4]) -> Option<Self> {
        let mut at = self.child;
        let mut result = None;
        while at != NO_LINK {
            let chunk = Self::read(data, at, self.end)?;
            at = chunk.sibling;
            if chunk.tag == tag && result.replace(chunk).is_some() {
                return None;
            }
        }
        result
    }
}

pub(super) fn decode_valley_forest(
    raw: &[u8],
    width: usize,
    height: usize,
    count: usize,
) -> Option<SourceForestGrid> {
    if !raw.starts_with(b"LChunkFile") || count != 335 {
        return None;
    }
    let version = Chunk::read(raw, 14, raw.len())?;
    if version.tag != b"Vers"
        || version.end != version.payload + 4
        || word(raw, version.payload)? != 434
        || version.child != NO_LINK
    {
        return None;
    }
    let root = Chunk::read(raw, version.sibling, raw.len())?;
    if root.tag != b"Wrld" || root.end != raw.len() || root.sibling != NO_LINK {
        return None;
    }
    let world = root.find_unique(raw, b"Wrld")?;
    let forest = world.find_unique(raw, b"UtFt")?;
    if forest.child != NO_LINK {
        return None;
    }
    let rows = &raw[forest.payload..forest.end];
    if [word(rows, 0)?, word(rows, 4)?, word(rows, 8)?] != [width, height, count] {
        return None;
    }
    let mut tail = &rows[12..];
    let mut grid = SourceForestGrid::new(width, height)?;
    for _ in 0..count {
        let [x, y, length] = [0, 2, 4].map(|at| {
            tail.get(at..at + 2)
                .and_then(|n| Some(u16::from_le_bytes(n.try_into().ok()?)))
        });
        let (x, y, length) = (x?, y?, usize::from(length?));
        let name = tail.get(6..6 + length)?;
        if !matches!(
            name,
            b"TRE_OAK"
                | b"TRE_MAPLE_02"
                | b"TRE_FIR"
                | b"TRE_OAK2"
                | b"TRE_MAPLE_01"
                | b"TRE_OAK_DEAD"
        ) || usize::from(x) >= 128 * width
            || usize::from(y) >= 128 * height
        {
            return None;
        }
        let trailer = tail.get(6 + length..)?;
        let flags = u16::from_le_bytes(trailer.get(..2)?.try_into().ok()?);
        let state = *trailer.get(2)?;
        if flags != 0x1a0 || state != 0 {
            return None;
        }
        grid.register([x, y], [TREE_EXTENT; 2], flags, state)?;
        tail = &trailer[3..];
    }
    tail.is_empty().then_some(grid)
}
