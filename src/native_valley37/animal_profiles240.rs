// Generated from the authenticated dbanima5.ddf and animals_15.ddf.
pub fn source_profile(name: &str) -> Option<AnimalProfile> {
    Some(match name {
        "Coyote" => AnimalProfile {
            hp: 20,
            corpse: None,
        },
        "Deer" => AnimalProfile {
            hp: 10,
            corpse: Some(CorpseType {
                name: "DeadAnimalDeer",
                animation: "sh_faudeer_death01.kf",
                food: Some(FoodStock {
                    amount: 160,
                    max_workers: 2,
                    exhaustible: true,
                }),
            }),
        },
        "Goat" => AnimalProfile {
            hp: 10,
            corpse: Some(CorpseType {
                name: "DeadAnimalGoat",
                animation: "sh_faugoat_death01.kf",
                food: Some(FoodStock {
                    amount: 80,
                    max_workers: 2,
                    exhaustible: true,
                }),
            }),
        },
        "Hawk" => AnimalProfile {
            hp: 60,
            corpse: Some(CorpseType {
                name: "DeadAnimalHawk",
                animation: "sh_fauhawk_death01.kf",
                food: None,
            }),
        },
        "Llama" => AnimalProfile {
            hp: 10,
            corpse: Some(CorpseType {
                name: "DeadAnimalLlama",
                animation: "sh_faullama_death01.kf",
                food: Some(FoodStock {
                    amount: 80,
                    max_workers: 2,
                    exhaustible: true,
                }),
            }),
        },
        "Tiger" => AnimalProfile {
            hp: 30,
            corpse: Some(CorpseType {
                name: "DeadAnimalTiger",
                animation: "sh_fautiger_death01.kf",
                food: Some(FoodStock {
                    amount: 10,
                    max_workers: 2,
                    exhaustible: true,
                }),
            }),
        },
        "Vulture" => AnimalProfile {
            hp: 60,
            corpse: Some(CorpseType {
                name: "DeadAnimalVulture",
                animation: "sh_fauvulture_death01.kf",
                food: None,
            }),
        },
        "WhiteWolf" => AnimalProfile {
            hp: 20,
            corpse: Some(CorpseType {
                name: "DeadAnimalWhiteWolf",
                animation: "sh_fauwolf_death01.kf",
                food: Some(FoodStock {
                    amount: 10,
                    max_workers: 2,
                    exhaustible: true,
                }),
            }),
        },
        "Wolf" => AnimalProfile {
            hp: 20,
            corpse: Some(CorpseType {
                name: "DeadAnimalWolf",
                animation: "sh_fauwolf_death01.kf",
                food: Some(FoodStock {
                    amount: 10,
                    max_workers: 2,
                    exhaustible: true,
                }),
            }),
        },
        _ => return None,
    })
}
