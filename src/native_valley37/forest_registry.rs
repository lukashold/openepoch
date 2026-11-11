#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForestId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForestRow {
    pub position: [u16; 3],
    pub profile: u8,
    pub flags: u16,
    pub state: u8,
}

impl ForestRow {
    pub fn position(&self) -> [f32; 3] {
        self.position.map(|word| f32::from(word) / 128.0)
    }
}

pub struct ForestRegistry {
    rows: Vec<Option<ForestRow>>,
}

impl ForestRegistry {
    pub fn new(rows: impl IntoIterator<Item = ForestRow>) -> Self {
        Self {
            rows: rows.into_iter().map(Some).collect(),
        }
    }

    pub fn get(&self, id: ForestId) -> Option<&ForestRow> {
        self.rows.get(id.0 as usize)?.as_ref()
    }

    pub fn take(&mut self, id: ForestId) -> Option<ForestRow> {
        self.rows.get_mut(id.0 as usize)?.take()
    }

    /// The caller supplies native broadphase order and the group-2 path check.
    pub fn select(
        &self,
        ordered: impl IntoIterator<Item = ForestId>,
        center: [f32; 2],
        radius: f32,
        extent: f32,
        mut reachable: impl FnMut([f32; 2]) -> bool,
    ) -> Option<ForestId> {
        let radius = f64::from(radius);
        let radius_squared = radius * radius;
        let offset = (f64::from(extent) * 0.9) as f32;
        let mut closest = f32::MAX;
        let mut selected = None;
        for id in ordered {
            let Some(tree) = self.get(id) else { continue };
            if tree.flags & 0x60 != 0x20 {
                continue;
            }
            let [x, y, _] = tree.position();
            let dx = f64::from(x) - f64::from(center[0]);
            let dy = f64::from(y) - f64::from(center[1]);
            let distance = (dx * dx + dy * dy) as f32;
            if f64::from(distance) > radius_squared || distance >= closest {
                continue;
            }
            let approaches = [
                [x + offset, y],
                [x - offset, y],
                [x, y + offset],
                [x, y - offset],
            ];
            if approaches.into_iter().any(&mut reachable) {
                closest = distance;
                selected = Some(id);
            }
        }
        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(profile: u8) -> ForestRow {
        ForestRow {
            position: [40 * 128, 40 * 128, 187],
            profile,
            flags: 0x1a0,
            state: 0,
        }
    }

    #[test]
    fn duplicate_positions_promote_once_without_reindexing() {
        let mut registry = ForestRegistry::new([row(0), row(1), row(2)]);
        let ordered = [ForestId(1), ForestId(0), ForestId(2)];
        let select =
            |registry: &ForestRegistry| registry.select(ordered, [40.0, 40.0], 5.0, 1.0, |_| true);
        assert_eq!(select(&registry), Some(ForestId(1)));
        assert_eq!(registry.take(ForestId(1)), Some(row(1)));
        assert_eq!(registry.take(ForestId(1)), None);
        assert_eq!(select(&registry), Some(ForestId(0)));
        assert_eq!(registry.get(ForestId(2)), Some(&row(2)));
        assert_eq!(registry.take(ForestId(u32::MAX)), None);
        assert_eq!(
            registry.take(ForestId(0)).map(|r| r.position()),
            Some([40.0, 40.0, 187.0 / 128.0])
        );
        assert_eq!(select(&registry), Some(ForestId(2)));
    }

    #[test]
    fn failed_path_keeps_source_rows_available() {
        let registry = ForestRegistry::new([row(2)]);
        let id = ForestId(0);
        let mut visited = 0;
        assert_eq!(
            registry.select([id], [40.0, 40.0], 5.0, 1.0, |_| {
                visited += 1;
                false
            }),
            None
        );
        assert_eq!(visited, 4);
        assert_eq!(registry.get(id), Some(&row(2)));
    }
}
