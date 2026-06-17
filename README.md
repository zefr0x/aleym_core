<div align = center>

<h1>Aleym | عَلِيم (Core Library)</h1>

[![release](https://github.com/zefr0x/aleym_core/actions/workflows/publish_release.yml/badge.svg)](https://github.com/zefr0x/aleym_core/actions/workflows/publish_release.yml)
[![test](https://github.com/zefr0x/aleym_core/actions/workflows/test.yml/badge.svg)](https://github.com/zefr0x/aleym_core/actions/workflows/test.yml)
[![lint](https://github.com/zefr0x/aleym_core/actions/workflows/lint.yml/badge.svg)](https://github.com/zefr0x/aleym_core/actions/workflows/lint.yml)
[![pre-commit.ci status](https://results.pre-commit.ci/badge/github/zefr0x/aleym_core/main.svg)](https://results.pre-commit.ci/latest/github/zefr0x/aleym_core/main)

This repository contain the **core library component of Aleym**. A game changing, feature-rich, and highly extensible
**news aggregation system and knowledge base** designed to streamline the process of news aggregation, prioritizing
organization, **efficiency**, **security**, and **privacy**.

The **feed aggregation engine** is implemented as a **library** that can be used to build a background service, serving
Aleym's functionalities to any front-end.

---

**[`Architecture`](./ARCHITECTURE.md)** | **[`Contribute`](./CONTRIBUTING.md)** | **[`Security`](./SECURITY.md)** |
**[`Q&A`](#qa)**

---

<br>

</div>

## Worth Noting Features

- Native **Tor** networking, enabling private fetching of feeds.
- Privacy respecting **randomized scheduler**, hiding timing patters (no static periodic fetching).
  - Experimental **smart scheduling** based on simple learning from previous fetches.
- Experimental **news recommendation** based on simple learning algorithms.

### Support News Informants

#### Standard

- [RSS](https://www.rssboard.org/rss-specification)
- [ATOM](https://en.wikipedia.org/wiki/Atom_(web_standard))
- [JSON Feed](https://www.jsonfeed.org/)

#### Nonstandard

##### Unofficial

- [Telegram Channels](https://telegram.org/tour/channels) (Web Scraper)

## Q&A

**Q:** What does `Aleym` mean?

- It is an Arabic word [`عَلِيم`](https://en.wiktionary.org/wiki/%D8%B9%D9%84%D9%8A%D9%85), means having great knowledge.

## License

<p>

<img src="https://www.gnu.org/graphics/agplv3-with-text-162x68.png" alt="AGPLv3 logo" align="right">
This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public
License as published by the Free Software Foundation, version 3 of the License only.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied
warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
details.

You should have received a copy of the GNU Affero General Public License along with this program. If not, see
<https://www.gnu.org/licenses/>.

</p>

## BibTeX

```bibtex
@software{aleym_core,
  author       = {zefr0x},
  title        = {Aleym Core},
  year         = {2026},
  url          = {https://github.com/zefr0x/aleym_core},
  license      = {AGPL-3.0-only},
}
```
