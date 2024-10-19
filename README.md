[Shields-io]: https://shields.io/badges/dynamic-json-badge  
[Nexus]: https://www.nexusmods.com  
[Nexus-key]: https://next.nexusmods.com/settings/api-keys  
[Git-key]: https://github.com/settings/tokens?type=beta  

# Nexus Badges
CLI tool to automate the process of creating and updating dynamic [shields.io][Shields-io] badges for displaying Download counts of mods hosted on [Nexus Mods][Nexus].
This program uses a private gist as a json endpoint for the dynamic badge to pull the download count from. The count for all tracked mods is stored within the same private
gist. Download counts are fetched directly from the Nexus api.  

## Prerequisites
- Find your Nexus 'Personal API Key' by logging in to Nexus Mods, then head to [next.nexusmods.com/settings/api-keys][Nexus-key]. Scrolling all the way to the bottom
  you will find your Personal key.
- Create a new git 'Fine-grained personal access token'. Ensure you are logged in to github and head to [github.com/settings/tokens][Git-key]
  1. Press 'Generate new token'
  2. Give the token a name and set its expiration date
  3. Expand the 'Account permissions' section
  4. Find sub-category 'Gists' and set to 'Read and Write'

<div align="center">
    <img src="https://i.imgur.com/1eNFHWu.png" width="80%">
</div>

## Usage
Download latest release of nexus_badges.exe or build from the source code. Use the `set-key` command to input your personal tokens.  

```
nexus_badges.exe set-key --git <GIT_TOKEN> --nexus <NEXUS_TOKEN>
```

<div align="center">  
  <img src="https://i.imgur.com/7OMHQc2.png" width="70%">  
</div>  

To track the download count of a nexus mod use the `add` command. Mods are tracked by their 'game_domain' and 'mod_id'. Locating them is easy by looking at the mod's url.  

```
nexus_badges.exe add -domain eldenring -mod-id 4825
```

To initialize the private gist that will store the download counts use the `init` command  

```
nexus_badges.exe init
```

After the initial setup is complete running the application will update the remote gist with the _current_ download counts for each tracked mod. Then store the proper
markdown of each badge in './io/badges.md'. Now you can copy your badges to your repository README.md or anywhere else that supports markdown. As long as you don't delete
the private gist you will not have to modify the badge. Just run the app whenever you want the count to be updated.  