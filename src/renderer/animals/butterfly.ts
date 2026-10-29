// Butterfly (Light egg): a sunny, fuzzy butterfly with curled antennae, a
// cream face and big spotted wings, and the chrysalis the caterpillar becomes
// on its way to Young (`PUPA`, shown by the pupa animation in evolve.ts).

import { px, type Animal } from "../art";

/** The chrysalis, hanging from a twig: shown while it turns into a butterfly. */
export const PUPA = px(`
  HHHHHHHHHHHH
  .....HH
  ....3333
  ...333333
  ..33a33a33
  ..33333333
  .3333oo3333
  .333333333c
  .33a3333a3c
  .333333333c
  ..3333333cc
  ..33a33a3c
  ...333333c
  ...33333cc
  ....3333c
  ....333c
  .....33c
  ......3
`);

export const butterfly: Animal = {
  young: px(`
    ......33.......33
    ........3.....3
    .........3...3
    .aaaa...3333333...aaaa
    aabbaa.333333333.aabbaa
    abbbba33wp333wp33abbbba
    abbbba33pp333pp33abbbba
    aabbaa33pe333pe33aabbaa
    .aaaaa333333M33333aaaaa
    ..aaaa.333333333.aaaa
    ..aaaaa.3ccccc3.aaaaa
    ..abba..3ccccc3..abba
    ..aaa...3ccccc3...aaa
    ........33333333
    .........3....3
  `),
  adult: px(`
    ........33............33
    ..........3..........3
    ...........3........3
    .aaaaaa.....3......3.....aaaaaa
    aabbbbaa.....3333333....aabbbbaa
    abbbbbbaa...333333333..aabbbbbba
    abbccbbbaa.33333333333.aabbbccbba
    abbccbbbaa3333wp333wp3333aabbbccbba
    abbbbbbbaa3333pp333pp3333aabbbbbbba
    aabbbbbaaa3333pe333pe3333aaabbbbbaa
    .aaaaaaaaa3333pe333pe3333aaaaaaaaa
    ..aaaaaaaa33333MMM333333aaaaaaaa
    ...aaaaaaaa33333333333aaaaaaaa
    ....aaaaaaa.333333333.aaaaaaa
    ....aaaaaa..3ccccccc3..aaaaaa
    ....aabbbaa.3ccccccc3.aabbbaa
    ....abbbbba.3ccccccc3.abbbbba
    ....aabbbaa.3ccccccc3.aabbbaa
    .....aaaaa..33ccccc33..aaaaa
    .............3333333
    ............33.....33
  `),
};
