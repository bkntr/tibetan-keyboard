# EWTS Conversion Reference

This is the complete input reference for Tibetan Wylie/EWTS Keyboard. It covers
every primitive conversion accepted by the live keyboard, plus the rules used
to combine those primitives into syllables and stacks.

Whole Tibetan syllables are compositional, and `+` permits arbitrary
non-standard stacks, so there is no finite list of every possible input string.
The tables below are exhaustive at the token level.

## Basic behavior

- Type consonants from top to bottom and put the vowel after the final
  consonant: `bsgribs` → `བསྒྲིབས`.
- A consonant has an inherent `a`. An explicit lowercase `a` after a consonant
  adds no mark: `ka` and `k` both produce `ཀ`.
- At the start of a syllable, `a` produces a-chen: `a` → `ཨ`.
- A regular space produces a Tibetan tsheg (`་`) and commits the current
  composition. Type `x` or `_` for an ordinary word space.
- Letter case matters. For example, `t` → `ཏ`, while `T` → `ཊ`.
- Backspace removes one source character and reconverts the current composition.

## Consonants

The base form is produced at the start of a stack. In a standard stack, or
after `+`, the same input can produce the subjoined form shown in the last
column. A combining mark in the table may appear attached to the preceding
character in some Markdown viewers.

| Type | Base result | Subjoined result | Notes |
| --- | --- | --- | --- |
| `k` | `ཀ` (U+0F40) | `ྐ` (U+0F90) | |
| `kh` | `ཁ` (U+0F41) | `ྑ` (U+0F91) | |
| `g` | `ག` (U+0F42) | `ྒ` (U+0F92) | |
| `gh`, `g+h` | `གྷ` | `ྒྷ` | Equivalent spellings |
| `ng` | `ང` (U+0F44) | `ྔ` (U+0F94) | |
| `c` | `ཅ` (U+0F45) | `ྕ` (U+0F95) | |
| `ch` | `ཆ` (U+0F46) | `ྖ` (U+0F96) | |
| `j` | `ཇ` (U+0F47) | `ྗ` (U+0F97) | |
| `ny` | `ཉ` (U+0F49) | `ྙ` (U+0F99) | |
| `T`, `-t` | `ཊ` (U+0F4A) | `ྚ` (U+0F9A) | Retroflex ta |
| `Th`, `-th` | `ཋ` (U+0F4B) | `ྛ` (U+0F9B) | Retroflex tha |
| `D`, `-d` | `ཌ` (U+0F4C) | `ྜ` (U+0F9C) | Retroflex da |
| `Dh`, `D+h`, `-dh`, `-d+h` | `ཌྷ` | `ྜྷ` | Retroflex dha |
| `N`, `-n` | `ཎ` (U+0F4E) | `ྞ` (U+0F9E) | Retroflex na |
| `t` | `ཏ` (U+0F4F) | `ྟ` (U+0F9F) | |
| `th` | `ཐ` (U+0F50) | `ྠ` (U+0FA0) | |
| `d` | `ད` (U+0F51) | `ྡ` (U+0FA1) | |
| `dh`, `d+h` | `དྷ` | `ྡྷ` | Equivalent spellings |
| `n` | `ན` (U+0F53) | `ྣ` (U+0FA3) | |
| `p` | `པ` (U+0F54) | `ྤ` (U+0FA4) | |
| `ph` | `ཕ` (U+0F55) | `ྥ` (U+0FA5) | |
| `b` | `བ` (U+0F56) | `ྦ` (U+0FA6) | |
| `bh`, `b+h` | `བྷ` | `ྦྷ` | Equivalent spellings |
| `m` | `མ` (U+0F58) | `ྨ` (U+0FA8) | |
| `ts` | `ཙ` (U+0F59) | `ྩ` (U+0FA9) | |
| `tsh` | `ཚ` (U+0F5A) | `ྪ` (U+0FAA) | |
| `dz` | `ཛ` (U+0F5B) | `ྫ` (U+0FAB) | |
| `dzh`, `dz+h` | `ཛྷ` | `ྫྷ` | Equivalent spellings |
| `w` | `ཝ` (U+0F5D) | `ྭ` (U+0FAD) | Lowercase wa |
| `zh` | `ཞ` (U+0F5E) | `ྮ` (U+0FAE) | |
| `z` | `ཟ` (U+0F5F) | `ྯ` (U+0FAF) | |
| `'` | `འ` (U+0F60) | `ྰ` (U+0FB0) | a-chung |
| `y` | `ཡ` (U+0F61) | `ྱ` (U+0FB1) | Lowercase ya |
| `r` | `ར` (U+0F62) | `ྲ` (U+0FB2) | Lowercase ra |
| `l` | `ལ` (U+0F63) | `ླ` (U+0FB3) | |
| `sh` | `ཤ` (U+0F64) | `ྴ` (U+0FB4) | |
| `Sh`, `-sh` | `ཥ` (U+0F65) | `ྵ` (U+0FB5) | Retroflex sha |
| `s` | `ས` (U+0F66) | `ྶ` (U+0FB6) | |
| `h` | `ཧ` (U+0F67) | `ྷ` (U+0FB7) | |
| `a` | `ཨ` (U+0F68) | `ྸ` (U+0FB8) | Acts as the inherent vowel after a consonant |
| `W` | `ཝ` (U+0F5D) | `ྺ` (U+0FBA) | Full-form wa in a lower position |
| `Y` | `ཡ` (U+0F61) | `ྻ` (U+0FBB) | Full-form ya in a lower position |
| `R` | `ཪ` (U+0F6A) | `ྼ` (U+0FBC) | Full-form ra |
| `f` | `ཕ༹` | — | pha with tsa-phru |
| `v` | `བ༹` | — | ba with tsa-phru |
| `&` | `྅` (U+0F85) | — | Tibetan paluta |

## Vowels

The result column shows the vowel sign. When a vowel begins a syllable, the
converter automatically prefixes a-chen (`ཨ`). The keyboard additionally accepts
the Tise/Denjong doubled spellings `aa`, `ii`, and `uu`.

| Type | Vowel sign | Example |
| --- | --- | --- |
| `a` | inherent vowel; no sign | `ka` → `ཀ`; `a` → `ཨ` |
| `A`, `aa` | `ཱ` (U+0F71) | `kA` or `kaa` → `ཀཱ` |
| `i` | `ི` (U+0F72) | `ki` → `ཀི` |
| `I`, `ii` | `ཱི` | `kI` or `kii` → `ཀཱི` |
| `u` | `ུ` (U+0F74) | `ku` → `ཀུ` |
| `U`, `uu` | `ཱུ` | `kU` or `kuu` → `ཀཱུ` |
| `e` | `ེ` (U+0F7A) | `ke` → `ཀེ` |
| `ai` | `ཻ` (U+0F7B) | `kai` → `ཀཻ` |
| `o` | `ོ` (U+0F7C) | `ko` → `ཀོ` |
| `au` | `ཽ` (U+0F7D) | `kau` → `ཀཽ` |
| `-i` | `ྀ` (U+0F80) | `k-i` → `ཀྀ` |
| `-I` | `ཱྀ` | `k-I` → `ཀཱྀ` |

## Final signs and marks

| Type | Result | Name/example |
| --- | --- | --- |
| `M` | `ཾ` (U+0F7E) | anusvara; `oM` → `ཨོཾ` |
| `` ~M` `` | `ྂ` (U+0F82) | nada bindu |
| `~M` | `ྃ` (U+0F83) | chandra bindu |
| `X` | `༷` (U+0F37) | nuqta |
| `~X` | `༵` (U+0F35) | chandra nuqta |
| `H` | `ཿ` (U+0F7F) | visarga |
| `?` | `྄` (U+0F84) | halanta/virama (srog med) |
| `^` | `༹` (U+0F39) | tsa-phru |

## Numbers, spaces, and punctuation

| Type | Result | Notes |
| --- | --- | --- |
| `0` | `༠` | Tibetan digit zero |
| `1` | `༡` | Tibetan digit one |
| `2` | `༢` | Tibetan digit two |
| `3` | `༣` | Tibetan digit three |
| `4` | `༤` | Tibetan digit four |
| `5` | `༥` | Tibetan digit five |
| `6` | `༦` | Tibetan digit six |
| `7` | `༧` | Tibetan digit seven |
| `8` | `༨` | Tibetan digit eight |
| `9` | `༩` | Tibetan digit nine |
| space | `་` | tsheg; commits the composition |
| `x`, `_` | ordinary space | `x` is a keyboard alias for EWTS `_`; commits the composition |
| `*` | `༌` | delimiter tsheg bstar |
| `/` | `།` | shad; commits the composition |
| `;` | `༏` | tsheg shad; commits the composition |
| `\|` | `༑` | rin chen spungs shad; commits the composition |
| `!` | `༈` | sbrul shad; commits the composition |
| `:` | `༔` | gter tsheg; commits the composition |
| `=` | `༴` | bsdus rtags |
| `<` | `༺` | gug rtags gyon |
| `>` | `༻` | gug rtags gyas |
| `(` | `༼` | ang khang gyon |
| `)` | `༽` | ang khang gyas |
| `@` | `༄` | initial yig mgo mdun ma |
| `#` | `༅` | closing yig mgo sgab ma |
| `$` | `༆` | caret yig mgo phur shad ma |
| `%` | `༇` | yig mgo tsheg shad ma |

Standard EWTS defines `//` → `༎` (nyis shad), but the live keyboard commits
the composition as soon as the first `/` is typed. Typing `//` therefore produces
two shads (`།།`), not `༎`.

The punctuation characters `"`, `,`, `` ` ``, `{`, and `}` are returned
unchanged. A hyphen or tilde is also returned unchanged when it does not begin
one of the multi-character tokens listed above.

## Stacks and composition operators

### Standard stacks

Standard Tibetan stacks form automatically without `+`. These are all of the
auto-stacking patterns supported by the converter:

| Pattern | Supported inputs |
| --- | --- |
| Superscribed `r` | `rka rga rnga rja rnya rta rda rna rba rma rtsa rdza` |
| Superscribed `l` | `lka lga lnga lca lja lta lda lpa lba lha` |
| Superscribed `s` | `ska sga snga snya sta sda sna spa sba sma stsa` |
| Subjoined `y` | `kya khya gya pya phya bya mya` |
| Subjoined `r` | `kra khra gra tra thra dra nra pra phra bra mra dzra shra sra hra` |
| Subjoined `l` | `kla gla bla rla sla zla` |
| Subjoined `w` | `kwa khwa gwa cwa nywa twa dwa tswa tshwa zhwa zwa rwa lwa shwa swa hwa` |
| Three-level stacks with `r` | `rkya rgya rmya rbwa rgwa rtswa` |
| Three-level stacks with `s` + `y` | `skya sgya spya sbya smya` |
| Three-level stacks with `s` + `r` | `skra sgra snra spra sbra smra` |
| Other three-level stacks | `grwa drwa phywa` |

Examples: `rgyas` → `རྒྱས`, `skyes` → `སྐྱེས`, and `grwa` →
`གྲྭ`.

### Operators

| Syntax | Effect | Example |
| --- | --- | --- |
| `+` between consonants | Forces a non-standard stack | `sat+t+wa` → `སཏྟྭ` |
| `+` in a standard stack | Optional; gives the same stack | `rta`, `r+ta` → `རྟ` |
| `.` between consonants | Prevents automatic stacking; emits nothing | `gyon` → `གྱོན`; `g.yon` → `གཡོན` |
| `+` between vowels | Places multiple vowel signs on one stack | `bru+e` → `བྲེུ` |

Lowercase `r`, `y`, and `w` select the normal superscribed/subjoined forms in
non-standard stacks. Uppercase `R`, `Y`, and `W` select their full forms. For
example, `r+R` → `རྼ` and `r+Y` → `རྻ`.

## Literal text and Unicode escapes

| Syntax | Effect | Example |
| --- | --- | --- |
| `[text]` | Inserts the enclosed text literally | `k[abc]` → `ཀabc` |
| `\c` | Inserts one ASCII character literally | `\3` → `3` |
| `\uXXXX` | Inserts a Unicode scalar from four hexadecimal digits | `\u0f40` → `ཀ` |
| `\UXXXXXXXX` | Inserts a Unicode scalar from eight hexadecimal digits | `\U00000f40` → `ཀ` |

The live keyboard admits all ASCII digits and punctuation, but it filters Latin
letters before conversion. The accepted letters are:

```text
abcdefghijklmnoprstuvwyzADHIMNRSTUWXY
```

Lowercase `q` and unsupported capital letters are ignored. Lowercase `x` always
becomes a word space. Those restrictions also apply inside bracketed text and
after a backslash; use the application's normal input mode for unrestricted
Latin text.

## Commit characters

The following inputs finish the current composition after their result is sent:

```text
space  _  /  ;  |  !  :  Enter
```

Lowercase `x` is converted to `_`, so it also commits. Clicking elsewhere,
using most navigation or shortcut keys, or disabling the keyboard also ends the
active composition without changing its rendered Tibetan text.
