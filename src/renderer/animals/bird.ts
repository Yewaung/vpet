// Bird (Wind egg): a round sky-blue bird with a swept crest, a cream breast,
// a golden beak and folded wings it beats to fly.

import { px, type Animal } from "../art";

export const bird: Animal = {
  young: px(`
    ...........333
    ..........33
    .........3333333
    ........3333333333
    .......333333333333
    .......3333wp333wp33
    .......3333pp333pp33yy
    .......3333pe333pe33yyyy
    ......33333pe333pe33YYY
    ......333333333333ccc
    .aa..33333333333ccccc
    aaaaaaa3333333cccccc
    .aaaaaaaa33333ccccccc
    ..aaaaaaaa333cccccccc
    ....aaaaa3333cccccc
    .......3333333ccccc
    ........33333333cc
    ..........YY..YY
    .........YYY.YYY
  `),
  adult: px(`
    ..............3333
    ............333..
    ...........33333333
    ...........3333333333
    ..........333333333333
    .........33333333333333
    .........3333wpp3333wp33
    .........3333ppp3333pp33yy
    .........3333pee3333pe33yyyyy
    .........3333pee3333pe33yyyy
    .........33333333333333YYY
    ........3333333333333cccc
    ........33333333333ccccccc
    .aa....3333333333ccccccccc
    aaaaa.33333333333cccccccccc
    .aaaaaaaa333333333ccccccccc
    ..aaaaaaaaaa33333cccccccccc
    ...aaaaaaaaaaa333ccccccccc
    ....aaaaaaaaaa33ccccccccc
    ......aaaaaaa333cccccccc
    .........33333333ccccc
    ..........3333333333
    ............YY...YY
    ...........YYYY.YYYY
  `),
};
