[status-img]: https://github.com/r0nd0ud0u/lib-rpg/actions/workflows/test.yml/badge.svg?branch=main
[status-url]: https://github.com/r0nd0ud0u/lib-rpg/actions/workflows/test.yml
[coverage-img]: https://img.shields.io/badge/Coverage-click--here-success?logo=github
[coverage-url]: https://r0nd0ud0u.github.io/lib-rpg/coverage/src/src/index.html

# Lib-RPG

[![Status][status-img]][status-url]
[![Coverage][coverage-img]][coverage-url]

This library defines the abstraction of the different functions to create a rpg game.

## Versions
- tag: 0.1.0 use  to call rust functions into cpp project
Example: https://github.com/r0nd0ud0u/qt-rpg/
- tag: from 1.0.0: cpp conversions not supported anymore, full rust interface 


## Description


The different functions and object can be called separetely.
If you want to create a game from scratch, there is 2 entrypoints: `GameManager` and `DataManager`.


```mermaid

flowchart TB

DataManager([DataManager])
GameManager([GameManager])
PlayerManager([<br>PlayerManager<br><br>])

GameManager-- <br> ---->PlayerManager

DataManager-- <br> ---->All-Heroes-characters
DataManager-- <br> ---->All-Bosses-characters

PlayerManager-- <br> ---->Selected-Heroes-characters
PlayerManager-- <br> ---->Selected-Bosses-characters

```

```mermaid

flowchart TB

Character([Character])

Character-- <br> ---->Stats
Character-- <br> ---->Description
Character-- <br> ---->Type:Hero-or-Boss
Character-- <br> ---->Equipment
Character-- <br> ---->Attacks
Character-- <br> ---->Level/experience
Character-- <br> ---->Fight-info

```

## Offlines
Offlines are defining the game.
Some are written in json format.
- characters
- equipments
- attaks

Images are stored as well.

## Contributing
- Issue -> PR
- Use of clippy and fmt
- Build: `cargo build`
- Test: `cargo test unit`
- Coverage: 
    - install tarpaulin `cargo install cargo-tarpaulin` 
    - run `cargo tarpaulin --out Lcov -- unit`

