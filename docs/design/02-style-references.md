# 02 · Style references

The visual references gathered on 2026-11-10 while choosing a look for the signup wizard
(`01-signup-wizard-spec.md`), with credits. The direction that came out of them is in
`03-style-directions.md`.

## Ground rules

- **Cues, not copies.** We study how these works look (line, contrast, colour, composition) and
  build our own. No characters, panels, covers, logos or interface layouts from them go into the
  app or this repo. This is `CLAUDE.md`'s IP constraint applied to every reference here, not only
  to Chaosium.
- **The images stay local.** Screenshots and covers are other people's copyright, and this repo
  is public. They live in `docs/design/references/`, which git ignores apart from its marker file.
  This doc carries the credits and source links, so the collection can be rebuilt from them.
- **Art we ship comes from two places only:** art we generate in code, and public-domain works we
  adapt and credit (below).

## The three references we kept

| Work | Creator | Publisher | Years | What we take from it |
|---|---|---|---|---|
| WORLD OF HORROR (game) | Paweł Koźmiński, working as panstasz | Ysbryd Games, PLAYISM | early access 2020; full release 19 Oct 2023 | 1-bit art and dithering; stats and events shown inside retro computer windows; dread from stillness |
| Tokyo Ghoul:re (manga, 16 volumes, plus the ZAKKI:re art book) | Sui Ishida (石田スイ) | Shueisha (Japan); VIZ Media (English) | 2014–2018 | Large white space, wet ink, a single red, restraint |
| BLAME! Master Edition (6 volumes) | Tsutomu Nihei (弐瓶勉) | Kodansha (Japan); Vertical Comics (English Master Edition) | series from 1997; Master Edition 2015–2017 | Pure black and white; vast architecture around a tiny figure; technical labelling |

Only the second series of Tokyo Ghoul (`:re`) is kept; the first series' books were looked at
and set aside. Only BLAME!'s Master Edition is kept because its covers are black and white: each
measured under 5% coloured pixels, against 10–78% for the original volume covers.

Sources:

- WORLD OF HORROR: [Steam store page](https://store.steampowered.com/app/913740/) (developer,
  publishers, release date); [Fangoria on the 1.0 release](https://www.fangoria.com/original/junji-ito-inspired-world-of-horror-release/)
  and [Fandom](https://www.fandom.com/articles/world-of-horror-1-bit) (drawn in MS Paint, inspired
  by Junji Ito and H. P. Lovecraft); [Gematsu](https://www.gematsu.com/games/world-of-horror)
  (early-access date).
- Tokyo Ghoul:re: [AniList series page](https://anilist.co/manga/85611) and
  [Sui Ishida on AniList](https://anilist.co/staff/110753); volumes on Open Library, for example
  [vol. 1](https://openlibrary.org/works/OL20043916W) and
  [ZAKKI:re](https://openlibrary.org/works/OL21684758W).
- BLAME!: [AniList series page](https://anilist.co/manga/30149) and
  [Tsutomu Nihei on AniList](https://anilist.co/staff/97000); Master Edition on Open Library, for
  example [vol. 1](https://openlibrary.org/works/OL19729937W) and
  [vol. 6](https://openlibrary.org/works/OL19764577W). Open Library also lists some volumes under
  other publishers' editions.

## Public-domain art we adapt

| Work | Artist | Date | Used for | Source |
|---|---|---|---|---|
| Kohada Koheiji, from the series Hyaku monogatari (One Hundred Ghost Stories) | Katsushika Hokusai | Edo period | The 1-bit ghost on style sheets A and A × B | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Hokusai_Kohada_Koheiji.jpg) |
| Inferno, Plate 9 (Canto III, Charon) | Gustave Doré | 1857, as dated on Commons | A 1-bit conversion test | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Gustave_Dor%C3%A9_-_Dante_Alighieri_-_Inferno_-_Plate_9_(Canto_III_-_Charon).jpg) |

Credit these wherever their adaptations appear (the app's About page is the natural place).

## Also explored, not carried forward

- **Games** (credits from each game's Steam page): Return of the Obra Dinn (Lucas Pope; 3909;
  2018), Darkest Dungeon (Red Hook Studios; 2016), Cultist Simulator (Weather Factory; 2018),
  Stygian: Reign of the Old Ones (Cultic Games; Fulqrum Publishing; 2019), Call of Cthulhu
  (Cyanide Studio; Focus Entertainment; 2018), SIGNALIS (rose-engine; Balor Games; 2022), Sunless
  Sea (Failbetter Games; 2015), Disco Elysium (ZA/UM; 2019), Inscryption (Daniel Mullins Games;
  Devolver Digital; 2021).
- **Animation and manga** (key visuals from each title's Wikipedia article): Uzumaki (Junji Ito),
  Junji Ito Maniac: Japanese Tales of the Macabre, Mononoke (Toei Animation), Yamishibai: Japanese
  Ghost Stories, Devilman Crybaby (based on Go Nagai's Devilman); the first Tokyo Ghoul series and
  its novels; Tsutomu Nihei's other series (NOiSE, Biomega, Abara, Knights of Sidonia, APOSIMZ,
  Tower Dungeon and others).
- **Public-domain prints and drawings** (Wikimedia Commons): Hokusai, Utagawa Kuniyoshi,
  Tsukioka Yoshitoshi, Toriyama Sekien, Harry Clarke, Gustave Doré, Francisco Goya, Odilon Redon,
  H. P. Lovecraft's own 1934 Cthulhu sketch, and two 1928 Weird Tales covers. If any of these is
  used later, check its credit on the Commons page first: two entries name the holding museum or
  the book's author where the artist should be.
- **Fan art** (DeviantArt): 14 Call of Cthulhu pieces and 8 Tokyo Ghoul pieces, each credited to
  its artist on the local contact sheet. The artists keep their copyright.

## Call of Cthulhu tools (links only)

Chaosium material is never copied here, not even into the local collection.

- [Chaosium's official character sheets](https://www.chaosium.com/cthulhu-character-sheets).
- [Dhole's House](https://www.dholeshouse.org/): a free online toolkit built by fan Lee Carnell
  and [featured on Chaosium's blog](https://chaosium.com/blog/welcome-to-the-dholes-house-free-online-toolkit-for-call-of-cthulhu).

## The local collection

`docs/design/references/` exists only on the machine that made it.

- `_contact.html`: the contact sheet. Hover to enlarge, click for the source. Serve the folder with
  the `style-references` entry in `.claude/launch.json` (local only) or any static file server.
- `_sections.json`: all 143 items with image links and credits; `_scripts/sections_view.json` is
  the current filtered view.
- `_scripts/`: the scripts that collected the items and build the sheet.
- `_ours/`: our 1-bit adaptations of the two public-domain prints.
