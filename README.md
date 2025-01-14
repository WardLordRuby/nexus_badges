[Shields-io]: https://shields.io/badges/dynamic-json-badge  
[Nexus]: https://www.nexusmods.com  
[Nexus-key]: https://next.nexusmods.com/settings/api-keys  
[Git-key]: https://github.com/settings/tokens?type=beta  
[Latest-dl]: https://github.com/WardLordRuby/nexus_badges/releases/latest
[automation]: .github/workflows/automation.yml
<div align="center">
    <img src="https://raw.githubusercontent.com/WardLordRuby/nexus_badges/refs/heads/main/assets/Icon_512.png" width="15%" height="15%">
</div>

# Nexus Badges
[![GitHub Downloads](https://img.shields.io/github/downloads/WardLordRuby/nexus_badges/total?label=Downloads&labelColor=%2323282e&color=%230e8726)][latest-dl]
[![GitHub License](https://img.shields.io/github/license/WardLordRuby/nexus_badges?label=License&labelColor=%2323282e)](LICENSE)  

Nexus Badges is a CLI tool that automates the process of creating and updating dynamic [shields.io][Shields-io] badges that display Download counts of mods hosted on
[Nexus Mods][Nexus]. This program uses a private gist as a json endpoint for the dynamic badge to pull the download count from. The count for all tracked mods is stored 
within the same private gist. Supports tracking of multiple Nexus Mods. Unique badges will be generated for each tracked mod. Download counts saved in the gist endpoint
are fetched directly from the Nexus api.  

## Compatibility
Nexus Badges is compatible with all major platforms. You can download releases for Windows, Linux, and macOS. If your target platform isn't listed, you can compile the 
source code directly for your desired system.

## Prerequisites
- Log into your Nexus Mods account and find your Nexus ['Personal API Key'][Nexus-key]. Scrolling all the way to the bottom
  of the linked page you will find your Personal key.
- **GitHub action automation users only:** Create a fork of this repository (or copy [automation.yml][automation] to the repo of your choosing)
- Ensure you are logged in to Github and create a new git ['Fine-grained personal access token'][Git-key].
  1. Press 'Generate new token'
  2. Give the token a name and set its expiration date
  3. **GitHub action automation users only:** Under 'Repository access' select 'Only select repositories' then choose your forked repository (or one containing [automation.yml][automation])
  4. Add permissions listed below based on how you want use Nexus Badges

<div align="center">
    
  | Permission Name     | Type       | Access level       | Required    | Required for GitHub action automation |
  |---------------------|------------|--------------------|------------:|--------------------------------------:|
  | Gists               | Account    | Read and write     |          ✅|                                     ✅|
  | Actions             | Repository | Read and write     |             |                                     ✅|
  | Secrets             | Repository | Read and write     |             |                                     ✅|
  | Variables           | Repository | Read and write     |             |                                     ✅|

</div>

## Initial set up
Download latest release of [nexus_badges][Latest-dl] or build from the source code. Use the `set-arg` command to input your personal tokens.  
```
nexus_badges.exe set-arg --git <GIT_TOKEN> --nexus <NEXUS_TOKEN>
```

<div align="center">  
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://i.imgur.com/T1wrzhk.png">
    <source media="(prefers-color-scheme: light)" srcset="https://i.imgur.com/yTUpMLH.png">
    <img src="https://i.imgur.com/888tw4j.png" width="70%">
  </picture>
</div>  

To track the download count of a nexus mod use the `add` command. Mods are tracked by their 'game_domain' and 'mod_id'. Locating them is easy by looking at the mod's url.  
```
nexus_badges.exe add --domain eldenring --mod-id 4825
```

To initialize the private gist that will store the download counts use the `init` command  
```
nexus_badges.exe init
```
### GitHub action automation set up  
Use the `set-arg` command again to locate the repository that contains [automation.yml][automation].
```
nexus_badges.exe set-arg --owner <GITHUB_NAME> --repo <REPOSITORY_NAME>
```
To initialize the automation workflow on your set repository use the `init-actons` command.
```
nexus_badges.exe init-actions
```
The automation workflow is set up to update the remote gist once a day.

## Normal usage
After the initial set up is complete running Nexus Badges will update the remote gist with the _current_ download counts for each tracked mod, then store the proper
markdown of each badge in './io/badges.md' or '~/Documents' depending on platform and installation type. Now you can copy your badges to your repository README.md or
anywhere else that supports the specified output format. As long as you don't delete the private gist you will not have to modify the badge. Just run Nexus Badges
whenever you want the count to be updated, or set up the Github action automation.

### Commands

<div align="center">

  | Commands             | Alias       | Description                                                                                  |
  | -------------------- | ----------- | -------------------------------------------------------------------------------------------- |
  | add                  | Add         | Add/Register a Nexus mod to track the download count of                                      |
  | remove               | Remove      | Remove and stop tracking the download count of a registered mod                              |
  | set-arg              | Set         | Configure necessary credentials and set badge style preferences                              |
  | init                 | Init        | Initialize private gist to be used as a json endpoint for badge download counters            |
  | init-actions         | Logs        | Initialize GitHub actions to update the remote gist once daily                               |
  | automation           | Automation  | Enable or disable the Github actions automation workflow [Possible values: enable, disable]  |
  | version              | Version     | Display current version and check for updates                                                |
  | help                 | -           | Displays helpful information                                                                 |

</div>

Each command has a help page access it with `nexus_badges.exe <COMMAND> --help`. Also note the initialize commands only need to be ran once. Every subsequent `add`, `remove`,
or `set-arg` command will take care of updating the remote gist endpoint and updating Github action workflow variables.  

### Badge stylization
Customize the badge output formatting and styling by using the following `set-arg`'s  

<div align="center">
    
  | Flag                        | Description                                                                                       |
  |-----------------------------|---------------------------------------------------------------------------------------------------|
  | `--style`                   | Badge style [Default: flat] [possible values: flat, flat-square, plastic, for-the-badge, social]  |
  | `--count`                   | Count to display [Default: total] [possible values: total, unique]                                |
  | `--label`                   | Badge label [Default: 'Nexus Downloads']                                                          |
  | `--color` & `--label-color` | Hex color for each side of the badge [Tip: input colors as `'#23282e'` or `23282e`]               |
  | `--format`                  | Badge output format [Default: Markdown] [possible values: markdown, url, rst, ascii-doc, html]    |

</div>
