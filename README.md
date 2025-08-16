![banner.png](assets/banner.png)

# Modrinth Collection Downloader 💾

This tool is used to download all mods from a Modrinth collection programatically. I built it for my own personal use but figured it was useful.

## Usage 🛠️

Just run the executable from [releases]("https://github.com/kay-xr/modrinth-collection-downloader/releases), and follow the prompts. 

## Compatibility Warnings ⚠️

This tool makes no assumptions for compatibility of the mods downloaded. If a mod requires dependencies, it is up to you to add these to your collection or download them separately manually.

This tool also assumes every mod will contain the version supplied. If a project does not contain a compatible version reported by the API, it will be skipped and a message will be shown at the end of the process.

## Modrinth API Notes 📝

The public Modrinth API only allows for 300 requests-per-minute. This should be enough when downloading a large list of mods, but downloads are throttled and may take extra time. 
