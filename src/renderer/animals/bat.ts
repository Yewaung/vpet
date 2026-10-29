// Bat (Dart egg): a round dusk-purple bat with huge ears, a tiny fang, a
// cream tummy and wide membrane wings it beats to fly.

import { px, type Animal } from "../art";

export const bat: Animal = {
  young: px(`
    ......3.........3
    ......33.......33
    ......3a3.....3a3
    ......3a33...3a33
    .a....3aa33333aa3.....a
    .aa...33333333333....aa
    .aaa.3333333333333..aaa
    .aaaa3333wpp333wp3.aaaa
    aaaaa3333ppp333pp3aaaaa
    aaaaa3333pee333pe3aaaaa
    aaaaa3333pee333pe3aaaaa
    .aaaa33333MhM33333aaaa
    ..aaa333333ccc3333aaa
    ...aa33333ccccc333aa
    ....a.333ccccc333.a
    ......3333ccc3333
    .......33.....33
  `),
  adult: px(`
    ..........3...........3
    ..........33.........33
    ..........3a3.......3a3
    ..........3aa3.....3aa3
    ..........3aa33333333aa3
    .a........33a333333333a3.........a
    .aa.......333333333333333.......aa
    .aaa.....33333wpp33333wp33.....aaa
    .aaaa....33333ppp33333pp33....aaaa
    .aaaaa...33333pee33333pe33...aaaaa
    aaaaaaa..33333pee33333pe33..aaaaaaa
    aaaaaaaa.333333333MhhM33333.aaaaaaaa
    aaaaaaaaa3333333333333333aaaaaaaaa
    .aaaaaaaa33333333cccc3333aaaaaaaa
    ..aaaaaaa3333333cccccc333aaaaaaa
    ...aaa.aa333333cccccccc33aa.aaa
    ....a...a33333cccccccc333a...a
    .........33333cccccccc333
    ..........33333cccccc333
    ...........333333333333
    ............33......33
    ...........333.....333
  `),
};
