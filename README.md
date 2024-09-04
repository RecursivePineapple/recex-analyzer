## recex-analyzer
### A recipe checker for recipe dumps from Gregtech: New Horizons

This tool checks for changes in recipes, recipe conflicts, new recipes, and removed recipes.

To do this, add the [RecEx](https://github.com/GTNewHorizons/RecEx) mod to your client, enter a world, press the 'k' button, and click the button to export all recipes to a json file. The file will be placed in the working directory for the client, under the RecEx-Records folder.

To check for conflicts, this tool may be ran with a single recipe dump. To check for additions and removals, this tool must be given two recipe dumps - one from before the changes and one from after. When comparing two dumps, the tool will also check for introduced and resolved conflicts.

### How it works
First, it will aggregate all GT recipes and group them by machine. Then, it will group them according to their input items & fluids (note: only item/fluid name, metadata, and amount are recorded in the dump). Once the recipes are grouped, the tool will compare them against the other dump if available. The tool will then determine the status of each recipe depending on a number of factors - mainly whether the recipe exists in the other dump, how many copies of the recipe exist, and what has changed within each recipe. Recipes that have no changes or conflicts are ignored completely.

#### Help text
```
Usage: recex-analyzer [OPTIONS] <BEFORE> [AFTER]

Arguments:
  <BEFORE>  Path to a recex dump prior to your changes
  [AFTER]   Path to a recex dump after to your changes. If this isn't set, only conflict analysis will be ran

Options:
  -o, --output <OUTPUT>        [default: analysis.json]
  -b, --blacklist <BLACKLIST>  [possible values: added, removed, outputs-changed, stats-changed, conflicting, conflict-created, conflict-removed, duplicate-registration]
  -w, --whitelist <WHITELIST>  [possible values: added, removed, outputs-changed, stats-changed, conflicting, conflict-created, conflict-removed, duplicate-registration]
  -h, --help                   Print help
  -V, --version                Print version
```
