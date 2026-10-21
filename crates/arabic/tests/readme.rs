//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use arabic::{Direction, reshape};

    let logical = reshape("السلام عليكم", Direction::Logical);
    assert_eq!(logical, "ﺍﻟﺴﻼﻡ ﻋﻠﻴﻜﻢ");

    let displayed = reshape("السلام عليكم", Direction::RightToLeft);
    assert_eq!(displayed, "ﻢﻜﻴﻠﻋ ﻡﺎﻠﺴﻟﺍ");
}
