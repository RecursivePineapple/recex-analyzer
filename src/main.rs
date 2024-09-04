use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::Read,
    path::PathBuf,
};

use clap::Parser;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct ItemStack {
    #[serde(alias = "a")]
    pub amount: i32,
    #[serde(alias = "m")]
    pub metadata: i32,
    #[serde(alias = "uN")]
    pub unlocalized_name: String,
    #[serde(alias = "lN")]
    pub localized_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct FluidStack {
    #[serde(alias = "a")]
    pub amount: i32,
    #[serde(alias = "uN")]
    pub unlocalized_name: String,
    #[serde(alias = "lN")]
    pub localized_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct GTRecipe {
    #[serde(alias = "en")]
    pub enabled: bool,
    #[serde(alias = "dur")]
    pub duration: i32,
    #[serde(alias = "eut")]
    pub eut: i32,
    #[serde(alias = "iI", skip_serializing_if = "Vec::is_empty")]
    pub item_inputs: Vec<ItemStack>,
    #[serde(alias = "fI", skip_serializing_if = "Vec::is_empty")]
    pub fluid_inputs: Vec<FluidStack>,
    #[serde(alias = "iO", skip_serializing_if = "Vec::is_empty")]
    pub item_outputs: Vec<ItemStack>,
    #[serde(alias = "fO", skip_serializing_if = "Vec::is_empty")]
    pub fluid_outputs: Vec<FluidStack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Machine {
    #[serde(alias = "n")]
    pub name: String,
    #[serde(alias = "recs", skip_serializing_if = "Vec::is_empty")]
    pub recipes: Vec<GTRecipe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShapedRecipe {
    #[serde(alias = "iI")]
    pub item_inputs: Vec<Option<ItemStack>>,
    #[serde(alias = "o")]
    pub item_output: ItemStack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShapelessRecipe {
    #[serde(alias = "iI")]
    pub item_inputs: HashSet<ItemStack>,
    #[serde(alias = "o")]
    pub item_output: ItemStack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OredictStack {
    #[serde(alias = "dns")]
    pub oredict_names: HashSet<String>,
    #[serde(alias = "ims")]
    pub candidates: HashSet<ItemStack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OredictInput {
    #[serde(flatten)]
    oredict: Option<OredictStack>,
    #[serde(flatten)]
    stack: Option<ItemStack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShapedOredictRecipe {
    #[serde(alias = "iI")]
    pub item_inputs: Vec<Option<OredictInput>>,
    #[serde(alias = "o")]
    pub item_output: ItemStack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum RecipeSource {
    #[serde(alias = "gregtech")]
    Gregtech { machines: Vec<Machine> },
    #[serde(alias = "shaped")]
    Shaped { recipes: Vec<ShapedRecipe> },
    #[serde(alias = "shapeless")]
    Shapeless { recipes: Vec<ShapelessRecipe> },
    #[serde(alias = "shapedOreDict")]
    ShapedOredict { recipes: Vec<ShapedOredictRecipe> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Root {
    sources: Vec<RecipeSource>,
}

impl Root {
    pub fn load(path: &PathBuf) -> Self {
        let mut s = String::new();

        println!("reading {path:?}");

        std::fs::OpenOptions::new()
            .read(true)
            .open(path)
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();

        println!("finished reading {path:?}");

        println!("loading {path:?}");

        let mut root: Self = serde_json::from_str(&s).unwrap();

        for source in &mut root.sources {
            match source {
                RecipeSource::Gregtech { machines } => {
                    for machine in machines {
                        for recipe in &mut machine.recipes {
                            recipe.item_inputs.sort();
                            recipe.fluid_inputs.sort();
                            recipe.item_outputs.sort();
                            recipe.fluid_outputs.sort();
                        }
                    }
                }
                _ => {}
            }
        }

        println!("finished loading {path:?}");

        root
    }

    pub fn get_gt_recipes(
        &self,
    ) -> HashMap<&String, HashMap<(Vec<ItemStack>, Vec<FluidStack>), Vec<&GTRecipe>>> {
        let gt = self
            .sources
            .iter()
            .find_map(|x| match x {
                RecipeSource::Gregtech { machines } => Some(machines),
                _ => None,
            })
            .unwrap();

        let mut per_machine = HashMap::new();

        for machine in gt {
            let mut by_inputs = HashMap::new();

            for recipe in &machine.recipes {
                let mut items = recipe.item_inputs.clone();
                let mut fluids = recipe.fluid_inputs.clone();

                items.sort();
                fluids.sort();

                let recipes: &mut Vec<_> = by_inputs
                    .entry((items, fluids))
                    .or_insert_with(|| Vec::new());

                recipes.push(recipe);
            }

            per_machine.insert(&machine.name, by_inputs);
        }

        per_machine
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, clap::ValueEnum)]
enum GTRecipeStatus {
    Added,
    Removed,
    OutputsChanged,
    StatsChanged,
    Conflicting,
    ConflictCreated,
    ConflictRemoved,
    DuplicateRegistration,
}

type RecipeKey = (Vec<ItemStack>, Vec<FluidStack>);
type RecipeLookup<'a> = HashMap<RecipeKey, Vec<&'a GTRecipe>>;
type RecipeMaps<'a> = HashMap<&'a String, RecipeLookup<'a>>;

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
enum RecipeBeforeAfter<'a> {
    Diff {
        before: Vec<&'a GTRecipe>,
        after: Vec<&'a GTRecipe>,
    },
    Same {
        recipes: Vec<&'a GTRecipe>,
    },
}

fn analyze<'a>(
    before: &'a RecipeMaps,
    after: &'a RecipeMaps,
) -> BTreeMap<&'a str, BTreeMap<GTRecipeStatus, Vec<RecipeBeforeAfter<'a>>>> {
    let mut statuses = BTreeMap::new();

    for (machine_name, after_recipes) in after {
        let before_recipes = match before.get(machine_name) {
            Some(x) => x,
            None => continue,
        };

        let keys = before_recipes
            .keys()
            .chain(after_recipes.keys())
            .collect::<HashSet<_>>();

        let mut machine_statuses = Vec::new();

        for key in keys {
            let before_recipe_list = before_recipes.get(key);
            let after_recipe_list = after_recipes.get(key);

            let before_has_conflict = before_recipe_list.map(|v| v.len() > 1).unwrap_or(false);
            let after_has_conflict = after_recipe_list.map(|v| v.len() > 1).unwrap_or(false);

            match (before_recipe_list, after_recipe_list) {
                (Some(b), None) => {
                    machine_statuses.push((
                        key.clone(),
                        b.iter().map(|x| *x).collect(),
                        Vec::new(),
                        GTRecipeStatus::Removed,
                    ));
                    continue;
                }
                (None, Some(a)) => {
                    machine_statuses.push((
                        key.clone(),
                        Vec::new(),
                        a.iter().map(|x| *x).collect(),
                        if after_has_conflict {
                            GTRecipeStatus::ConflictCreated
                        } else {
                            GTRecipeStatus::Added
                        },
                    ));
                    continue;
                }
                _ => {}
            }

            let before_recipe_list: Vec<&GTRecipe> =
                before_recipe_list.unwrap().iter().map(|x| *x).collect();
            let after_recipe_list: Vec<&GTRecipe> =
                after_recipe_list.unwrap().iter().map(|x| *x).collect();

            if before_has_conflict || after_has_conflict {
                if before_has_conflict && !after_has_conflict {
                    machine_statuses.push((
                        key.clone(),
                        before_recipe_list,
                        after_recipe_list,
                        GTRecipeStatus::ConflictRemoved,
                    ));
                    continue;
                }

                if !before_has_conflict && after_has_conflict {
                    machine_statuses.push((
                        key.clone(),
                        before_recipe_list,
                        after_recipe_list,
                        GTRecipeStatus::ConflictCreated,
                    ));
                    continue;
                }

                let first = before_recipe_list.get(0).unwrap();

                let all_recipes_the_same = before_recipe_list
                    .iter()
                    .chain(after_recipe_list.iter())
                    .all(|r| r == first);

                machine_statuses.push((
                    key.clone(),
                    before_recipe_list,
                    after_recipe_list,
                    if all_recipes_the_same {
                        GTRecipeStatus::DuplicateRegistration
                    } else {
                        GTRecipeStatus::Conflicting
                    },
                ));
                continue;
            }

            let before_recipe = *before_recipe_list.get(0).unwrap();
            let after_recipe = *after_recipe_list.get(0).unwrap();

            if before_recipe.fluid_outputs != after_recipe.fluid_outputs
                || before_recipe.item_outputs != after_recipe.item_outputs
            {
                machine_statuses.push((
                    key.clone(),
                    before_recipe_list,
                    after_recipe_list,
                    GTRecipeStatus::OutputsChanged,
                ));
                continue;
            }

            if before_recipe.duration != after_recipe.duration
                || before_recipe.eut != after_recipe.eut
                || before_recipe.enabled != after_recipe.enabled
            {
                machine_statuses.push((
                    key.clone(),
                    before_recipe_list,
                    after_recipe_list,
                    GTRecipeStatus::StatsChanged,
                ));
                continue;
            }
        }

        if !machine_statuses.is_empty() {
            statuses.insert(
                machine_name.as_str(),
                machine_statuses
                    .into_iter()
                    .map(|(_, mut before, mut after, status)| {
                        (status, {
                            before.sort();
                            after.sort();

                            if before == after {
                                RecipeBeforeAfter::Same { recipes: before }
                            } else {
                                RecipeBeforeAfter::Diff { before, after }
                            }
                        })
                    })
                    .into_group_map()
                    .into_iter()
                    .map(|(k, mut v)| {
                        v.sort();
                        (k, v)
                    })
                    .collect::<BTreeMap<_, _>>(),
            );
        }
    }

    statuses
}

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "analysis.json")]
    output: PathBuf,

    #[doc = "Path to a recex dump prior to your changes"]
    #[arg()]
    before: PathBuf,

    #[doc = "Path to a recex dump after to your changes."]
    #[doc = "If this isn't set, only conflict analysis will be ran."]
    #[arg(required = false)]
    after: Option<PathBuf>,

    #[arg(short = 'b', long)]
    blacklist: Vec<GTRecipeStatus>,

    #[arg(short = 'w', long)]
    whitelist: Vec<GTRecipeStatus>,
}

fn main() {
    let args = Args::parse();

    if args.blacklist.len() > 0 && args.whitelist.len() > 0 {
        panic!("cannot use --blacklist and --whitelist at the same time");
    }

    let before = Root::load(&args.before);
    let after = match &args.after {
        Some(x) => Root::load(x),
        None => before.clone(),
    };

    println!("finding gt recipes");

    let before_gt = before.get_gt_recipes();
    let after_gt = after.get_gt_recipes();

    println!("analyzing recipes");

    let mut status = analyze(&before_gt, &after_gt);

    for (_, machine) in &mut status {
        if args.blacklist.len() > 0 {
            machine.retain(|k, _| !args.blacklist.contains(k));
        }

        if args.whitelist.len() > 0 {
            machine.retain(|k, _| args.whitelist.contains(k));
        }
    }

    println!("writing {:?}", args.output);

    let status = serde_json::to_string_pretty(&status).unwrap();

    std::fs::write(&args.output, status).unwrap();
}
