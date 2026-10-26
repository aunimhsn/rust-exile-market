# Images

The Grinding Gear Games [developer guidelines](https://www.pathofexile.com/developer/docs) forbid using their intellectual property to make money, imitating their official platforms with their logos, and presenting a tool as made or approved by them; every public tool must show the sentence "This product isn't affiliated with or endorsed by Grinding Gear Games in any way." Showing the game icons in a free tool fits these rules, like poe2db, poe.ninja and poe2scout do. The application follows these rules:

- `assets/currencies/` holds the Exalted, Divine and Chaos Orbs, the only game icons shown. They are embedded in the executable, so that they show offline. [assets/NOTICE.md](../assets/NOTICE.md) states that they belong to Grinding Gear Games and are not covered by the project license.
- The window shows no item icon: no other game image is downloaded or added to the repository, and none is extracted from the game files.
- `assets/logo/` holds the logo and the emblem of the application, original creations of the project, without the Path of Exile logo.
- The disclaimer is shown in the README and in the application settings.
