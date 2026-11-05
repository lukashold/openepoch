//! Logical state of the 004F25A0 build-list loader and 004F2E40 tech iterator.
//! Inputs represent one invocation of the native AIP reader. Resolving file
//! includes, reader flags and the lookup of a named row array is the reader's
//! responsibility.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name(String);

impl Name {
    pub fn new(value: &str) -> Option<Self> {
        // Supported AIP names are ASCII C strings in a 32-byte field.
        if !value.is_ascii() || value.len() >= 32 || value.contains('\0') {
            return None;
        }
        Some(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Context {
    Standard,
    NewCity,
    HotSpot,
}

impl Context {
    const ORDER: [Self; 3] = [Self::Standard, Self::NewCity, Self::HotSpot];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildRow {
    pub unit: Name,
    pub priority_bits: u32,
    pub hints: String,
    pub block: i32,
    pub desired_min: i32,
    pub desired_max: i32,
    pub build_max: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildList {
    pub name: Name,
    pub context: Context,
    pub rows: Vec<BuildRow>,
}

/// All slices are the results of the current reader's lookup. A zero count
/// covers both an empty declaration and a lookup that returned no rows.
pub trait AipRead {
    fn registry(&self, context: Context) -> &[Name];
    fn build_rows(&self, registry_name: &Name) -> &[BuildRow];
    fn tech_names(&self) -> &[Name];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerData {
    registries: [Vec<Name>; 3],
    lists: Vec<BuildList>,
    tech_names: Vec<Name>,
    tech_cursor: i32,
}

impl Default for PlannerData {
    fn default() -> Self {
        Self {
            registries: [Vec::new(), Vec::new(), Vec::new()],
            lists: Vec::new(),
            tech_names: Vec::new(),
            tech_cursor: -1,
        }
    }
}

impl PlannerData {
    pub fn apply(&mut self, reader: &impl AipRead) {
        // 004F1730 replaces all three name registries before refreshing lists.
        for context in Context::ORDER {
            let names = reader.registry(context);
            if !names.is_empty() {
                self.registries[context as usize] = names.to_vec();
            }
        }
        for context in Context::ORDER {
            for name in &self.registries[context as usize] {
                let index = match self
                    .lists
                    .iter()
                    .position(|entry| entry.name.as_str().eq_ignore_ascii_case(name.as_str()))
                {
                    Some(index) => index,
                    None => {
                        self.lists.push(BuildList {
                            name: name.clone(),
                            context,
                            rows: Vec::new(),
                        });
                        self.lists.len() - 1
                    }
                };
                let entry = &mut self.lists[index];
                let rows = reader.build_rows(name);
                if !rows.is_empty() {
                    entry.rows = rows.to_vec();
                }
                entry.context = context;
            }
        }
        let technologies = reader.tech_names();
        if !technologies.is_empty() {
            self.tech_names = technologies.to_vec();
        }
        self.rewind_technologies();
    }

    pub fn registry(&self, context: Context) -> &[Name] {
        &self.registries[context as usize]
    }

    pub fn lists(&self) -> &[BuildList] {
        &self.lists
    }

    pub fn tech_names(&self) -> &[Name] {
        &self.tech_names
    }

    pub fn tech_cursor(&self) -> i32 {
        self.tech_cursor
    }

    pub(crate) fn rewind_technologies(&mut self) {
        self.tech_cursor = -1;
    }

    /// 004F2E40 copies the advanced name into an LString output slot.
    /// Exhaustion returns false and leaves both cursor and output untouched.
    pub fn next_technology(&mut self, output: &mut Option<Name>) -> bool {
        if self.tech_cursor >= self.tech_names.len() as i32 - 1 {
            return false;
        }
        self.tech_cursor += 1;
        *output = usize::try_from(self.tech_cursor)
            .ok()
            .and_then(|index| self.tech_names.get(index))
            .cloned();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct ReadFrame {
        registries: [Vec<Name>; 3],
        lists: Vec<(Name, Vec<BuildRow>)>,
        tech_names: Vec<Name>,
    }

    impl AipRead for ReadFrame {
        fn registry(&self, context: Context) -> &[Name] {
            &self.registries[context as usize]
        }

        fn build_rows(&self, name: &Name) -> &[BuildRow] {
            self.lists
                .iter()
                .find(|(key, _)| key == name)
                .map_or(&[], |(_, rows)| rows.as_slice())
        }

        fn tech_names(&self) -> &[Name] {
            &self.tech_names
        }
    }

    fn name(value: &str) -> Name {
        Name::new(value).unwrap()
    }

    fn row(unit: &str, build_max: i32) -> BuildRow {
        BuildRow {
            unit: name(unit),
            priority_bits: 1.125_f32.to_bits(),
            hints: "lowThreat".to_owned(),
            block: 1,
            desired_min: 1,
            desired_max: 7,
            build_max,
        }
    }

    #[test]
    fn named_list_identity_is_shared_across_contexts_and_keeps_nonempty_rows() {
        let mut data = PlannerData::default();
        let input = ReadFrame {
            registries: [
                vec![name("core"), name("absent")],
                vec![name("CORE")],
                vec![name("Core")],
            ],
            lists: vec![
                (name("core"), vec![row("Citizen", 10)]),
                (name("CORE"), vec![row("Barracks", 3)]),
            ],
            ..ReadFrame::default()
        };
        data.apply(&input);
        assert_eq!(
            data.lists(),
            [
                BuildList {
                    name: name("core"),
                    context: Context::HotSpot,
                    rows: vec![row("Barracks", 3)]
                },
                BuildList {
                    name: name("absent"),
                    context: Context::Standard,
                    rows: vec![]
                },
            ]
        );

        let mut patch = ReadFrame::default();
        patch.registries[Context::Standard as usize] = vec![name("other")];
        patch.registries[Context::HotSpot as usize] = vec![name("other")];
        data.apply(&patch);
        assert_eq!(data.registry(Context::NewCity), [name("CORE")]);
        assert_eq!(data.lists()[0].context, Context::NewCity);
        assert_eq!(data.lists()[0].rows, [row("Barracks", 3)]);
        assert!(!Context::ORDER
            .iter()
            .any(|&context| data.registry(context).contains(&name("absent"))));
        assert_eq!(data.lists()[1].name, name("absent"));
        assert_eq!(data.lists()[2].name, name("other"));
    }

    #[test]
    fn tech_iteration_preserves_output_on_exhaustion_and_restarts_after_empty_apply() {
        let mut data = PlannerData::default();
        let mut output = Some(name("sentinel"));
        assert!(!data.next_technology(&mut output));
        assert_eq!(output, Some(name("sentinel")));
        data.apply(&ReadFrame {
            tech_names: vec![
                name("Agriculture"),
                name("Agriculture"),
                name("EarlyCommerce"),
            ],
            ..ReadFrame::default()
        });
        for technology in ["Agriculture", "Agriculture", "EarlyCommerce"] {
            assert!(data.next_technology(&mut output));
            assert_eq!(output, Some(name(technology)));
        }
        assert_eq!(data.tech_cursor(), 2);
        assert!(!data.next_technology(&mut output));
        assert_eq!(output, Some(name("EarlyCommerce")));
        data.apply(&ReadFrame::default());
        assert_eq!(data.tech_cursor(), -1);
        assert!(data.next_technology(&mut output));
        assert_eq!(output, Some(name("Agriculture")));
    }
}
