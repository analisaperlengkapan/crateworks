# arabic

Arabic text shaping: contextual letter forms, ligatures, harakat handling and
normalization. No dependencies.

```rust
use arabic::{Direction, reshape};

// "peace be upon you" in Arabic, shaped but still in logical order.
let logical = reshape("السلام عليكم", Direction::Logical);
assert_eq!(logical, "ﺍﻟﺴﻼﻡ ﻋﻠﻴﻜﻢ");

// For a display that draws left to right, the shaped text is reversed.
let displayed = reshape("السلام عليكم", Direction::RightToLeft);
assert_eq!(displayed, "ﻢﻜﻴﻠﻋ ﻡﺎﻠﺴﻟﺍ");
```

Arabic letters change shape depending on their neighbours, and terminals, older
renderers and PDF generators expect the text to arrive already shaped. This
crate does that: each letter is replaced by the presentation form that matches
its neighbours, in logical order or reversed for display.

## Shaping and normalising are different jobs

Shaping prepares text for display and must be applied last. Normalising
prepares text for matching and should be applied to what you store and index:

```rust
use arabic::normalize;

// Both spellings of the same word compare equal.
assert_eq!(normalize("أحمد"), normalize("احمد"));
```

`normalize` removes combining marks and tatweel, unifies the alef, yeh, heh and
kaf forms, and collapses whitespace. Its output is a lookup key, not display
text.

## Options

`reshape_with` takes an `Options` value. `Options::default()` keeps combining
marks and tatweel and applies the standard ligatures; `Options::plain()` drops
marks and tatweel for displays that cannot position them.

Only the ligatures in ordinary use are applied: the four lam-alef combinations
and the word Allah. Unicode defines hundreds more, but substituting them by
default produces text that most fonts cannot render.

Part of the [crateworks](https://github.com/analisaperlengkapan/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
