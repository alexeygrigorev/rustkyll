//! jemoji plugin: convert `:emoji_name:` patterns to GitHub emoji `<img>` tags.
//!
//! This module implements the `jemoji` Jekyll plugin behavior. When the plugin
//! is listed in a site's `plugins` or `gems` configuration, `:shortcode:` patterns
//! in rendered HTML are converted to `<img class="emoji">` tags pointing to
//! GitHub's emoji CDN.
//!
//! Shortcodes inside `<code>`, `<pre>`, `<tt>`, `<script>`, and `<style>` tags
//! are NOT converted. Shortcodes inside HTML attributes are NOT converted.

use crate::config::SiteConfig;

/// Check if the site has `jemoji` in its `plugins` or `gems` list.
pub fn has_jemoji_plugin(config: &SiteConfig) -> bool {
    for key in &["plugins", "gems"] {
        if let Some(val) = config.extras.get(*key) {
            if let Some(seq) = val.as_sequence() {
                if seq
                    .iter()
                    .any(|v| v.as_str().map(|s| s == "jemoji").unwrap_or(false))
                {
                    return true;
                }
            }
        }
    }
    false
}

/// GitHub emoji CDN base URL.
const EMOJI_CDN_BASE: &str = "https://github.githubassets.com/images/icons/emoji/unicode";

/// Process HTML content, replacing `:shortcode:` patterns with emoji `<img>` tags.
///
/// This function:
/// - Skips content inside `<code>`, `<pre>`, `<tt>`, `<script>`, `<style>` tags
/// - Skips content inside HTML attributes
/// - Only replaces known shortcodes (unknown ones are left as-is)
/// - Uses GitHub's emoji CDN URLs
pub fn process_jemoji(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    // Track nesting depth of tags where emoji should not be processed.
    let mut skip_depth: usize = 0;

    while i < len {
        if bytes[i] == b'<' {
            // Parse the tag - find the closing >
            if let Some(tag_end) = find_tag_end(bytes, i) {
                let tag_content = &html[i..=tag_end];
                let tag_lower = tag_content.to_ascii_lowercase();

                if is_opening_skip_tag(&tag_lower) {
                    skip_depth += 1;
                } else if is_closing_skip_tag(&tag_lower) {
                    skip_depth = skip_depth.saturating_sub(1);
                }

                result.push_str(tag_content);
                i = tag_end + 1;
                continue;
            }
        }

        if skip_depth > 0 || bytes[i] != b':' {
            // Safe to push byte-by-byte for ASCII, but we need to handle multi-byte UTF-8.
            // Use char boundary approach.
            let ch = html[i..].chars().next().unwrap();
            result.push(ch);
            i += ch.len_utf8();
            continue;
        }

        // We have a ':' outside skip zones and outside tags.
        // Try to parse :shortcode: pattern where shortcode is [a-z0-9_+-]+
        if let Some((name, end_pos)) = parse_shortcode(bytes, i) {
            if let Some(codepoint) = emoji_codepoint(name) {
                result.push_str(&format!(
                    "<img class=\"emoji\" title=\":{}:\" alt=\":{}:\" src=\"{}/{}.png\" height=\"20\" width=\"20\">",
                    name, name, EMOJI_CDN_BASE, codepoint
                ));
                i = end_pos;
                continue;
            }
        }

        result.push(':');
        i += 1;
    }

    result
}

/// Try to parse a `:shortcode:` pattern starting at position `start`.
/// Returns the shortcode name (as &str) and the byte position after the closing `:`.
fn parse_shortcode(bytes: &[u8], start: usize) -> Option<(&str, usize)> {
    let len = bytes.len();
    if start + 2 >= len || bytes[start] != b':' {
        return None;
    }

    let name_start = start + 1;
    // First character must be [a-z0-9_+]
    if !is_shortcode_char(bytes[name_start]) {
        return None;
    }

    let mut j = name_start;
    while j < len && is_shortcode_char(bytes[j]) {
        j += 1;
    }

    // Must have at least one character and end with ':'
    if j == name_start || j >= len || bytes[j] != b':' {
        return None;
    }

    let name = std::str::from_utf8(&bytes[name_start..j]).ok()?;
    Some((name, j + 1))
}

/// Check if a byte is a valid shortcode character: [a-z0-9_+-]
fn is_shortcode_char(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'+' || b == b'-'
}

/// Find the index of the closing `>` for a tag starting at `start`.
fn find_tag_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    let mut in_quote = false;
    let mut quote_char = b'"';

    while i < bytes.len() {
        if in_quote {
            if bytes[i] == quote_char {
                in_quote = false;
            }
        } else {
            match bytes[i] {
                b'"' | b'\'' => {
                    in_quote = true;
                    quote_char = bytes[i];
                }
                b'>' => return Some(i),
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Check if a lowercase tag string is an opening tag for protected elements.
fn is_opening_skip_tag(tag: &str) -> bool {
    is_tag_match(tag, "<code", 5)
        || is_tag_match(tag, "<pre", 4)
        || is_tag_match(tag, "<tt", 3)
        || is_tag_match(tag, "<script", 7)
        || is_tag_match(tag, "<style", 6)
}

/// Check if `tag` starts with `prefix` and the character after the prefix
/// is a space or `>` (i.e., it's a real tag, not a prefix of something else).
fn is_tag_match(tag: &str, prefix: &str, prefix_len: usize) -> bool {
    tag.starts_with(prefix)
        && (tag.len() == prefix_len
            || tag
                .as_bytes()
                .get(prefix_len)
                .is_some_and(|&b| b == b' ' || b == b'>'))
}

/// Check if a lowercase tag string is a closing tag for protected elements.
fn is_closing_skip_tag(tag: &str) -> bool {
    tag.starts_with("</code>")
        || tag.starts_with("</pre>")
        || tag.starts_with("</tt>")
        || tag.starts_with("</script>")
        || tag.starts_with("</style>")
}

/// Apply jemoji processing to HTML if the plugin is enabled in config.
///
/// Returns the HTML unchanged if `jemoji` is not in the plugins list.
pub fn apply_jemoji_if_enabled(html: &str, config: &SiteConfig) -> String {
    if has_jemoji_plugin(config) {
        process_jemoji(html)
    } else {
        html.to_string()
    }
}

/// Look up the Unicode codepoint(s) for an emoji shortcode name.
/// Returns None if the shortcode is not recognized.
fn emoji_codepoint(name: &str) -> Option<&'static str> {
    // Comprehensive GitHub emoji map (gemoji data).
    // Maps shortcode names to Unicode codepoint hex strings as used in GitHub CDN URLs.
    match name {
        // Smileys & Emotion
        "grinning" => Some("1f600"),
        "grimacing" => Some("1f62c"),
        "grin" => Some("1f601"),
        "joy" => Some("1f602"),
        "rofl" => Some("1f923"),
        "partying_face" => Some("1f973"),
        "smiley" => Some("1f603"),
        "smile" => Some("1f604"),
        "sweat_smile" => Some("1f605"),
        "laughing" | "satisfied" => Some("1f606"),
        "innocent" => Some("1f607"),
        "wink" => Some("1f609"),
        "blush" => Some("1f60a"),
        "slightly_smiling_face" => Some("1f642"),
        "upside_down_face" => Some("1f643"),
        "relaxed" => Some("263a"),
        "yum" => Some("1f60b"),
        "relieved" => Some("1f60c"),
        "heart_eyes" => Some("1f60d"),
        "smiling_face_with_three_hearts" => Some("1f970"),
        "kissing_heart" => Some("1f618"),
        "kissing" => Some("1f617"),
        "kissing_smiling_eyes" => Some("1f619"),
        "kissing_closed_eyes" => Some("1f61a"),
        "stuck_out_tongue_winking_eye" => Some("1f61c"),
        "zany_face" => Some("1f92a"),
        "stuck_out_tongue_closed_eyes" => Some("1f61d"),
        "stuck_out_tongue" => Some("1f61b"),
        "money_mouth_face" => Some("1f911"),
        "hugs" => Some("1f917"),
        "nerd_face" => Some("1f913"),
        "sunglasses" => Some("1f60e"),
        "star_struck" => Some("1f929"),
        "clown_face" => Some("1f921"),
        "cowboy_hat_face" => Some("1f920"),
        "shushing_face" => Some("1f92b"),
        "thinking" => Some("1f914"),
        "zipper_mouth_face" => Some("1f910"),
        "raised_eyebrow" => Some("1f928"),
        "neutral_face" => Some("1f610"),
        "expressionless" => Some("1f611"),
        "no_mouth" => Some("1f636"),
        "smirk" => Some("1f60f"),
        "unamused" => Some("1f612"),
        "roll_eyes" => Some("1f644"),
        "face_with_monocle" => Some("1f9d0"),
        "grimace" => Some("1f62c"),
        "lying_face" => Some("1f925"),
        "pensive" => Some("1f614"),
        "sleepy" => Some("1f62a"),
        "sleeping" => Some("1f634"),
        "drooling_face" => Some("1f924"),
        "mask" => Some("1f637"),
        "face_with_thermometer" => Some("1f912"),
        "face_with_head_bandage" => Some("1f915"),
        "nauseated_face" => Some("1f922"),
        "sneezing_face" => Some("1f927"),
        "hot_face" => Some("1f975"),
        "cold_face" => Some("1f976"),
        "woozy_face" => Some("1f974"),
        "dizzy_face" => Some("1f635"),
        "exploding_head" => Some("1f92f"),
        "face_with_cowboy_hat" => Some("1f920"),
        "hushed" => Some("1f62f"),
        "astonished" => Some("1f632"),
        "flushed" => Some("1f633"),
        "pleading_face" => Some("1f97a"),
        "fearful" => Some("1f628"),
        "cold_sweat" => Some("1f630"),
        "disappointed_relieved" => Some("1f625"),
        "cry" => Some("1f622"),
        "sob" => Some("1f62d"),
        "scream" => Some("1f631"),
        "confounded" => Some("1f616"),
        "persevere" => Some("1f623"),
        "disappointed" => Some("1f61e"),
        "sweat" => Some("1f613"),
        "weary" => Some("1f629"),
        "tired_face" => Some("1f62b"),
        "yawning_face" => Some("1f971"),
        "triumph" => Some("1f624"),
        "rage" | "pout" => Some("1f621"),
        "angry" => Some("1f620"),
        "cursing_face" => Some("1f92c"),
        "smiling_imp" => Some("1f608"),
        "imp" => Some("1f47f"),
        "skull" => Some("1f480"),
        "skull_and_crossbones" => Some("2620"),
        "hankey" | "poop" | "shit" => Some("1f4a9"),
        "ghost" => Some("1f47b"),
        "alien" => Some("1f47e"),
        "robot" => Some("1f916"),
        "smiley_cat" => Some("1f63a"),
        "smile_cat" => Some("1f638"),
        "joy_cat" => Some("1f639"),
        "heart_eyes_cat" => Some("1f63b"),
        "smirk_cat" => Some("1f63c"),
        "kissing_cat" => Some("1f63d"),
        "scream_cat" => Some("1f640"),
        "crying_cat_face" => Some("1f63f"),
        "pouting_cat" => Some("1f63e"),
        "see_no_evil" => Some("1f648"),
        "hear_no_evil" => Some("1f649"),
        "speak_no_evil" => Some("1f64a"),

        // Gestures & Body
        "wave" => Some("1f44b"),
        "raised_back_of_hand" => Some("1f91a"),
        "raised_hand_with_fingers_splayed" => Some("1f590"),
        "hand" | "raised_hand" => Some("270b"),
        "vulcan_salute" => Some("1f596"),
        "ok_hand" => Some("1f44c"),
        "pinching_hand" => Some("1f90f"),
        "v" => Some("270c"),
        "crossed_fingers" => Some("1f91e"),
        "love_you_gesture" => Some("1f91f"),
        "metal" => Some("1f918"),
        "call_me_hand" => Some("1f919"),
        "point_left" => Some("1f448"),
        "point_right" => Some("1f449"),
        "point_up_2" => Some("1f446"),
        "middle_finger" | "fu" => Some("1f595"),
        "point_down" => Some("1f447"),
        "point_up" => Some("261d"),
        "+1" | "thumbsup" => Some("1f44d"),
        "-1" | "thumbsdown" => Some("1f44e"),
        "fist_raised" | "fist" => Some("270a"),
        "fist_oncoming" | "facepunch" | "punch" => Some("1f44a"),
        "fist_left" => Some("1f91b"),
        "fist_right" => Some("1f91c"),
        "clap" => Some("1f44f"),
        "raised_hands" => Some("1f64c"),
        "open_hands" => Some("1f450"),
        "palms_up_together" => Some("1f932"),
        "handshake" => Some("1f91d"),
        "pray" => Some("1f64f"),
        "writing_hand" => Some("270d"),
        "nail_care" => Some("1f485"),
        "selfie" => Some("1f933"),
        "muscle" => Some("1f4aa"),
        "leg" => Some("1f9b5"),
        "foot" => Some("1f9b6"),
        "ear" => Some("1f442"),
        "nose" => Some("1f443"),
        "eyes" => Some("1f440"),
        "eye" => Some("1f441"),
        "tongue" => Some("1f445"),
        "lips" => Some("1f444"),
        "brain" => Some("1f9e0"),
        "bone" => Some("1f9b4"),
        "tooth" => Some("1f9b7"),

        // People
        "baby" => Some("1f476"),
        "boy" => Some("1f466"),
        "girl" => Some("1f467"),
        "man" => Some("1f468"),
        "woman" => Some("1f469"),
        "older_man" => Some("1f474"),
        "older_woman" => Some("1f475"),
        "cop" | "police_officer" => Some("1f46e"),
        "construction_worker" => Some("1f477"),
        "princess" => Some("1f478"),
        "prince" => Some("1f934"),
        "angel" => Some("1f47c"),
        "santa" => Some("1f385"),
        "mrs_claus" => Some("1f936"),
        "bow" => Some("1f647"),
        "information_desk_person" | "tipping_hand_person" => Some("1f481"),
        "no_good" | "no_good_woman" => Some("1f645"),
        "ok_woman" | "ok_person" => Some("1f646"),
        "raising_hand" | "raising_hand_woman" => Some("1f64b"),
        "deaf_person" => Some("1f9cf"),
        "person_facepalming" | "woman_facepalming" => Some("1f926"),
        "person_shrugging" | "woman_shrugging" => Some("1f937"),
        "dancer" | "woman_dancing" => Some("1f483"),
        "man_dancing" => Some("1f57a"),
        "couple" => Some("1f46b"),
        "two_men_holding_hands" | "men_holding_hands" => Some("1f46c"),
        "two_women_holding_hands" | "women_holding_hands" => Some("1f46d"),
        "family" => Some("1f46a"),

        // Hearts & Emotions
        "heart" => Some("2764"),
        "orange_heart" => Some("1f9e1"),
        "yellow_heart" => Some("1f49b"),
        "green_heart" => Some("1f49a"),
        "blue_heart" => Some("1f499"),
        "purple_heart" => Some("1f49c"),
        "black_heart" => Some("1f5a4"),
        "broken_heart" => Some("1f494"),
        "heavy_heart_exclamation" => Some("2763"),
        "two_hearts" => Some("1f495"),
        "revolving_hearts" => Some("1f49e"),
        "heartbeat" => Some("1f493"),
        "heartpulse" => Some("1f497"),
        "sparkling_heart" => Some("1f496"),
        "cupid" => Some("1f498"),
        "gift_heart" => Some("1f49d"),
        "heart_decoration" => Some("1f49f"),
        "love_letter" => Some("1f48c"),
        "white_heart" => Some("1f90d"),
        "brown_heart" => Some("1f90e"),

        // Symbols & Signs
        "100" => Some("1f4af"),
        "anger" => Some("1f4a2"),
        "boom" | "collision" => Some("1f4a5"),
        "dizzy" => Some("1f4ab"),
        "sweat_drops" => Some("1f4a6"),
        "dash" => Some("1f4a8"),
        "hole" => Some("1f573"),
        "bomb" => Some("1f4a3"),
        "speech_balloon" => Some("1f4ac"),
        "thought_balloon" => Some("1f4ad"),
        "zzz" => Some("1f4a4"),
        "exclamation" | "heavy_exclamation_mark" => Some("2757"),
        "question" => Some("2753"),
        "grey_exclamation" => Some("2755"),
        "grey_question" => Some("2754"),
        "bangbang" => Some("203c"),
        "interrobang" => Some("2049"),
        "x" => Some("274c"),
        "o" => Some("2b55"),
        "white_check_mark" => Some("2705"),
        "heavy_check_mark" => Some("2714"),
        "heavy_multiplication_x" => Some("2716"),
        "warning" => Some("26a0"),
        "no_entry" => Some("26d4"),
        "no_entry_sign" => Some("1f6ab"),

        // Celebration & Party
        "tada" => Some("1f389"),
        "confetti_ball" => Some("1f38a"),
        "balloon" => Some("1f388"),
        "birthday" => Some("1f382"),
        "gift" => Some("1f381"),
        "ribbon" => Some("1f380"),
        "sparkles" => Some("2728"),
        "sparkler" => Some("1f387"),
        "fireworks" => Some("1f386"),
        "firecracker" => Some("1f9e8"),
        "trophy" => Some("1f3c6"),
        "medal_sports" => Some("1f3c5"),
        "medal_military" => Some("1f396"),
        "1st_place_medal" => Some("1f947"),
        "2nd_place_medal" => Some("1f948"),
        "3rd_place_medal" => Some("1f949"),
        "crown" => Some("1f451"),

        // Nature & Animals
        "dog" => Some("1f436"),
        "cat" => Some("1f431"),
        "mouse" => Some("1f42d"),
        "hamster" => Some("1f439"),
        "rabbit" => Some("1f430"),
        "fox_face" => Some("1f98a"),
        "bear" => Some("1f43b"),
        "panda_face" => Some("1f43c"),
        "koala" => Some("1f428"),
        "tiger" => Some("1f42f"),
        "lion" => Some("1f981"),
        "cow" => Some("1f42e"),
        "pig" => Some("1f437"),
        "frog" => Some("1f438"),
        "monkey_face" => Some("1f435"),
        "monkey" => Some("1f412"),
        "chicken" => Some("1f414"),
        "penguin" => Some("1f427"),
        "bird" => Some("1f426"),
        "eagle" => Some("1f985"),
        "owl" => Some("1f989"),
        "bat" => Some("1f987"),
        "wolf" => Some("1f43a"),
        "horse" => Some("1f434"),
        "unicorn" => Some("1f984"),
        "bee" | "honeybee" => Some("1f41d"),
        "bug" => Some("1f41b"),
        "butterfly" => Some("1f98b"),
        "snail" => Some("1f40c"),
        "lady_beetle" => Some("1f41e"),
        "ant" => Some("1f41c"),
        "mosquito" => Some("1f99f"),
        "cricket" => Some("1f997"),
        "spider" => Some("1f577"),
        "spider_web" => Some("1f578"),
        "scorpion" => Some("1f982"),
        "turtle" => Some("1f422"),
        "snake" => Some("1f40d"),
        "lizard" => Some("1f98e"),
        "dragon_face" => Some("1f432"),
        "dragon" => Some("1f409"),
        "sauropod" => Some("1f995"),
        "t_rex" | "t-rex" => Some("1f996"),
        "whale" => Some("1f433"),
        "whale2" => Some("1f40b"),
        "dolphin" | "flipper" => Some("1f42c"),
        "fish" => Some("1f41f"),
        "tropical_fish" => Some("1f420"),
        "blowfish" => Some("1f421"),
        "shark" => Some("1f988"),
        "octopus" => Some("1f419"),
        "shell" => Some("1f41a"),
        "crab" => Some("1f980"),
        "lobster" => Some("1f99e"),
        "shrimp" => Some("1f990"),
        "squid" => Some("1f991"),

        // Plants & Flowers
        "bouquet" => Some("1f490"),
        "cherry_blossom" => Some("1f338"),
        "white_flower" => Some("1f4ae"),
        "rosette" => Some("1f3f5"),
        "rose" => Some("1f339"),
        "wilted_flower" => Some("1f940"),
        "hibiscus" => Some("1f33a"),
        "sunflower" => Some("1f33b"),
        "blossom" => Some("1f33c"),
        "tulip" => Some("1f337"),
        "seedling" => Some("1f331"),
        "evergreen_tree" => Some("1f332"),
        "deciduous_tree" => Some("1f333"),
        "palm_tree" => Some("1f334"),
        "cactus" => Some("1f335"),
        "shamrock" => Some("2618"),
        "four_leaf_clover" => Some("1f340"),
        "herb" => Some("1f33f"),
        "fallen_leaf" => Some("1f342"),
        "leaves" => Some("1f343"),
        "maple_leaf" => Some("1f341"),
        "mushroom" => Some("1f344"),
        "ear_of_rice" => Some("1f33e"),
        "christmas_tree" => Some("1f384"),
        "tanabata_tree" => Some("1f38b"),
        "bamboo" => Some("1f38d"),

        // Food & Drink
        "apple" => Some("1f34e"),
        "green_apple" => Some("1f34f"),
        "pear" => Some("1f350"),
        "tangerine" | "orange" | "mandarin" => Some("1f34a"),
        "lemon" => Some("1f34b"),
        "banana" => Some("1f34c"),
        "watermelon" => Some("1f349"),
        "grapes" => Some("1f347"),
        "strawberry" => Some("1f353"),
        "melon" => Some("1f348"),
        "cherries" => Some("1f352"),
        "peach" => Some("1f351"),
        "mango" => Some("1f96d"),
        "pineapple" => Some("1f34d"),
        "coconut" => Some("1f965"),
        "kiwi_fruit" => Some("1f95d"),
        "tomato" => Some("1f345"),
        "eggplant" => Some("1f346"),
        "avocado" => Some("1f951"),
        "broccoli" => Some("1f966"),
        "carrot" => Some("1f955"),
        "corn" => Some("1f33d"),
        "hot_pepper" => Some("1f336"),
        "potato" => Some("1f954"),
        "sweet_potato" => Some("1f360"),
        "chestnut" => Some("1f330"),
        "peanuts" => Some("1f95c"),
        "honey_pot" => Some("1f36f"),
        "bread" => Some("1f35e"),
        "croissant" => Some("1f950"),
        "pizza" => Some("1f355"),
        "hamburger" => Some("1f354"),
        "fries" => Some("1f35f"),
        "hotdog" => Some("1f32d"),
        "sandwich" => Some("1f96a"),
        "taco" => Some("1f32e"),
        "burrito" => Some("1f32f"),
        "egg" => Some("1f95a"),
        "coffee" => Some("2615"),
        "tea" => Some("1f375"),
        "beer" => Some("1f37a"),
        "beers" => Some("1f37b"),
        "wine_glass" => Some("1f377"),
        "cocktail" => Some("1f378"),
        "tropical_drink" => Some("1f379"),
        "champagne" => Some("1f37e"),
        "sake" => Some("1f376"),
        "ice_cream" => Some("1f368"),
        "cake" => Some("1f370"),
        "cookie" => Some("1f36a"),
        "chocolate_bar" => Some("1f36b"),
        "candy" => Some("1f36c"),
        "lollipop" => Some("1f36d"),
        "doughnut" => Some("1f369"),

        // Travel & Places
        "earth_africa" => Some("1f30d"),
        "earth_americas" => Some("1f30e"),
        "earth_asia" => Some("1f30f"),
        "globe_with_meridians" => Some("1f310"),
        "world_map" => Some("1f5fa"),
        "house" => Some("1f3e0"),
        "house_with_garden" => Some("1f3e1"),
        "office" => Some("1f3e2"),
        "hospital" => Some("1f3e5"),
        "school" => Some("1f3eb"),
        "church" => Some("26ea"),
        "rocket" => Some("1f680"),
        "airplane" => Some("2708"),
        "car" | "red_car" => Some("1f697"),
        "taxi" => Some("1f695"),
        "bus" => Some("1f68c"),
        "train" => Some("1f686"),
        "ship" => Some("1f6a2"),
        "anchor" => Some("2693"),

        // Objects
        "gem" => Some("1f48e"),
        "ring" => Some("1f48d"),
        "bulb" => Some("1f4a1"),
        "flashlight" => Some("1f526"),
        "wrench" => Some("1f527"),
        "hammer" => Some("1f528"),
        "nut_and_bolt" => Some("1f529"),
        "gear" => Some("2699"),
        "link" => Some("1f517"),
        "chains" => Some("26d3"),
        "scissors" => Some("2702"),
        "lock" => Some("1f512"),
        "unlock" => Some("1f513"),
        "key" => Some("1f511"),
        "bell" => Some("1f514"),
        "no_bell" => Some("1f515"),
        "bookmark" => Some("1f516"),
        "books" => Some("1f4da"),
        "book" | "open_book" => Some("1f4d6"),
        "notebook" => Some("1f4d3"),
        "notebook_with_decorative_cover" => Some("1f4d4"),
        "ledger" => Some("1f4d2"),
        "closed_book" => Some("1f4d5"),
        "green_book" => Some("1f4d7"),
        "blue_book" => Some("1f4d8"),
        "orange_book" => Some("1f4d9"),
        "memo" | "pencil" => Some("1f4dd"),
        "pencil2" => Some("270f"),
        "pen" => Some("1f58a"),
        "email" | "envelope" => Some("2709"),
        "incoming_envelope" => Some("1f4e8"),
        "outbox_tray" => Some("1f4e4"),
        "inbox_tray" => Some("1f4e5"),
        "package" => Some("1f4e6"),
        "mailbox" => Some("1f4eb"),
        "calendar" => Some("1f4c5"),
        "date" => Some("1f4c5"),
        "clipboard" => Some("1f4cb"),
        "pushpin" => Some("1f4cc"),
        "paperclip" => Some("1f4ce"),
        "file_folder" => Some("1f4c1"),
        "open_file_folder" => Some("1f4c2"),
        "computer" => Some("1f4bb"),
        "desktop_computer" => Some("1f5a5"),
        "keyboard" => Some("2328"),
        "mouse_computer" | "computer_mouse" => Some("1f5b1"),
        "printer" => Some("1f5a8"),
        "floppy_disk" => Some("1f4be"),
        "cd" => Some("1f4bf"),
        "dvd" => Some("1f4c0"),
        "camera" => Some("1f4f7"),
        "movie_camera" => Some("1f3a5"),
        "tv" => Some("1f4fa"),
        "telephone_receiver" => Some("1f4de"),
        "phone" | "telephone" => Some("260e"),
        "iphone" => Some("1f4f1"),
        "battery" => Some("1f50b"),
        "electric_plug" => Some("1f50c"),
        "mag" => Some("1f50d"),
        "mag_right" => Some("1f50e"),
        "hourglass" => Some("231b"),
        "hourglass_flowing_sand" => Some("23f3"),
        "watch" => Some("231a"),
        "alarm_clock" => Some("23f0"),
        "stopwatch" => Some("23f1"),
        "timer_clock" => Some("23f2"),
        "clock12" => Some("1f55b"),
        "fire" | "flame" => Some("1f525"),
        "star" => Some("2b50"),
        "star2" => Some("1f31f"),
        "zap" => Some("26a1"),
        "sunny" => Some("2600"),
        "cloud" => Some("2601"),
        "rainbow" => Some("1f308"),
        "snowflake" => Some("2744"),
        "umbrella" => Some("2614"),
        "ocean" => Some("1f30a"),
        "droplet" => Some("1f4a7"),

        // Flags and misc
        "checkered_flag" => Some("1f3c1"),
        "triangular_flag_on_post" => Some("1f6a9"),
        "crossed_flags" => Some("1f38c"),
        "black_flag" => Some("1f3f4"),
        "white_flag" => Some("1f3f3"),
        "rainbow_flag" => Some("1f3f3-fe0f-200d-1f308"),
        "pirate_flag" => Some("1f3f4-200d-2620-fe0f"),

        // Misc symbols
        "recycle" => Some("267b"),
        "copyright" => Some("00a9"),
        "registered" => Some("00ae"),
        "tm" => Some("2122"),
        "hash" => Some("0023-20e3"),
        "keycap_star" => Some("002a-20e3"),
        "zero" => Some("0030-20e3"),
        "one" => Some("0031-20e3"),
        "two" => Some("0032-20e3"),
        "three" => Some("0033-20e3"),
        "four" => Some("0034-20e3"),
        "five" => Some("0035-20e3"),
        "six" => Some("0036-20e3"),
        "seven" => Some("0037-20e3"),
        "eight" => Some("0038-20e3"),
        "nine" => Some("0039-20e3"),
        "ten" => Some("1f51f"),

        // Arrows
        "arrow_up" => Some("2b06"),
        "arrow_down" => Some("2b07"),
        "arrow_left" => Some("2b05"),
        "arrow_right" => Some("27a1"),
        "arrow_upper_left" => Some("2196"),
        "arrow_upper_right" => Some("2197"),
        "arrow_lower_left" => Some("2199"),
        "arrow_lower_right" => Some("2198"),
        "left_right_arrow" => Some("2194"),
        "arrow_up_down" => Some("2195"),
        "leftwards_arrow_with_hook" => Some("21a9"),
        "arrow_right_hook" => Some("21aa"),
        "arrow_heading_up" => Some("2934"),
        "arrow_heading_down" => Some("2935"),
        "arrows_clockwise" => Some("1f503"),
        "arrows_counterclockwise" => Some("1f504"),
        "back" => Some("1f519"),
        "end" => Some("1f51a"),
        "on" => Some("1f51b"),
        "soon" => Some("1f51c"),
        "top" => Some("1f51d"),

        // Music & entertainment
        "musical_note" => Some("1f3b5"),
        "notes" | "musical_notes" => Some("1f3b6"),
        "musical_score" => Some("1f3bc"),
        "guitar" => Some("1f3b8"),
        "microphone" => Some("1f3a4"),
        "headphones" => Some("1f3a7"),
        "saxophone" => Some("1f3b7"),
        "trumpet" => Some("1f3ba"),
        "drum" => Some("1f941"),
        "violin" => Some("1f3bb"),
        "art" => Some("1f3a8"),
        "game_die" => Some("1f3b2"),
        "dart" => Some("1f3af"),
        "bowling" => Some("1f3b3"),
        "video_game" => Some("1f3ae"),
        "slot_machine" => Some("1f3b0"),
        "jigsaw" => Some("1f9e9"),

        // Sports
        "soccer" => Some("26bd"),
        "basketball" => Some("1f3c0"),
        "football" => Some("1f3c8"),
        "baseball" => Some("26be"),
        "tennis" => Some("1f3be"),
        "volleyball" => Some("1f3d0"),
        "golf" | "golfing" => Some("1f3cc"),
        "ping_pong" => Some("1f3d3"),
        "running" | "runner" => Some("1f3c3"),

        // Clothing
        "eyeglasses" => Some("1f453"),
        "dark_sunglasses" => Some("1f576"),
        "necktie" => Some("1f454"),
        "shirt" | "tshirt" => Some("1f455"),
        "jeans" => Some("1f456"),
        "dress" => Some("1f457"),
        "kimono" => Some("1f458"),
        "bikini" => Some("1f459"),
        "womans_clothes" => Some("1f45a"),
        "purse" => Some("1f45b"),
        "handbag" => Some("1f45c"),
        "pouch" => Some("1f45d"),
        "mans_shoe" | "shoe" => Some("1f45e"),
        "athletic_shoe" => Some("1f45f"),
        "high_heel" => Some("1f460"),
        "sandal" => Some("1f461"),
        "boot" => Some("1f462"),
        "tophat" => Some("1f3a9"),
        "mortar_board" => Some("1f393"),

        // Zodiac
        "aries" => Some("2648"),
        "taurus" => Some("2649"),
        "gemini" => Some("264a"),
        "cancer" => Some("264b"),
        "leo" => Some("264c"),
        "virgo" => Some("264d"),
        "libra" => Some("264e"),
        "scorpius" => Some("264f"),
        "sagittarius" => Some("2650"),
        "capricorn" => Some("2651"),
        "aquarius" => Some("2652"),
        "pisces" => Some("2653"),

        // Misc
        "mega" => Some("1f4e3"),
        "loudspeaker" => Some("1f4e2"),
        "mute" => Some("1f507"),
        "speaker" => Some("1f508"),
        "sound" => Some("1f509"),
        "loud_sound" => Some("1f50a"),
        "rotating_light" => Some("1f6a8"),
        "sos" => Some("1f198"),
        "stop_sign" => Some("1f6d1"),
        "construction" => Some("1f6a7"),
        "information_source" => Some("2139"),
        "peace_symbol" => Some("262e"),
        "atom_symbol" => Some("269b"),
        "beginner" => Some("1f530"),
        "trident" => Some("1f531"),
        "infinity" => Some("267e"),
        "eye_speech_bubble" => Some("1f441-200d-1f5e8"),

        // Common extras
        "thumbsup_tone1" => Some("1f44d-1f3fb"),
        "thumbsup_tone2" => Some("1f44d-1f3fc"),
        "thumbsup_tone3" => Some("1f44d-1f3fd"),
        "thumbsup_tone4" => Some("1f44d-1f3fe"),
        "thumbsup_tone5" => Some("1f44d-1f3ff"),
        "octocat" => Some("octocat"), // Special GitHub emoji, not unicode
        "shipit" => Some("shipit"),   // Special GitHub emoji, not unicode

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_config_with_plugins(plugins: Vec<&str>) -> SiteConfig {
        let mut extras = HashMap::new();
        let plugins_val: Vec<serde_yaml::Value> = plugins
            .into_iter()
            .map(|s| serde_yaml::Value::String(s.to_string()))
            .collect();
        extras.insert(
            "plugins".to_string(),
            serde_yaml::Value::Sequence(plugins_val),
        );
        SiteConfig {
            extras,
            ..SiteConfig::default()
        }
    }

    fn make_config_with_gems(gems: Vec<&str>) -> SiteConfig {
        let mut extras = HashMap::new();
        let gems_val: Vec<serde_yaml::Value> = gems
            .into_iter()
            .map(|s| serde_yaml::Value::String(s.to_string()))
            .collect();
        extras.insert("gems".to_string(), serde_yaml::Value::Sequence(gems_val));
        SiteConfig {
            extras,
            ..SiteConfig::default()
        }
    }

    // --- Plugin detection tests ---

    #[test]
    fn test_jemoji_plugin_detected_in_plugins() {
        let config = make_config_with_plugins(vec!["jemoji", "jekyll-feed"]);
        assert!(has_jemoji_plugin(&config));
    }

    #[test]
    fn test_jemoji_plugin_detected_in_gems() {
        let config = make_config_with_gems(vec!["jemoji"]);
        assert!(has_jemoji_plugin(&config));
    }

    #[test]
    fn test_jemoji_plugin_detected_among_many() {
        let config = make_config_with_plugins(vec!["jekyll-feed", "jemoji", "jekyll-seo-tag"]);
        assert!(has_jemoji_plugin(&config));
    }

    #[test]
    fn test_jemoji_plugin_not_detected_when_absent() {
        let config = make_config_with_plugins(vec!["jekyll-feed", "jekyll-seo-tag"]);
        assert!(!has_jemoji_plugin(&config));
    }

    #[test]
    fn test_jemoji_plugin_not_detected_empty_config() {
        let config = SiteConfig::default();
        assert!(!has_jemoji_plugin(&config));
    }

    // --- Emoji shortcode replacement tests ---

    #[test]
    fn test_heart_emoji() {
        let html = "<p>I :heart: this</p>";
        let result = process_jemoji(html);
        assert_eq!(
            result,
            "<p>I <img class=\"emoji\" title=\":heart:\" alt=\":heart:\" src=\"https://github.githubassets.com/images/icons/emoji/unicode/2764.png\" height=\"20\" width=\"20\"> this</p>"
        );
    }

    #[test]
    fn test_white_check_mark_emoji() {
        let html = "<p>:white_check_mark: Done</p>";
        let result = process_jemoji(html);
        assert!(result.contains(
            "src=\"https://github.githubassets.com/images/icons/emoji/unicode/2705.png\""
        ));
        assert!(result.contains("class=\"emoji\""));
        assert!(result.contains("title=\":white_check_mark:\""));
    }

    #[test]
    fn test_x_emoji() {
        let html = "<p>:x: Failed</p>";
        let result = process_jemoji(html);
        assert!(result.contains(
            "src=\"https://github.githubassets.com/images/icons/emoji/unicode/274c.png\""
        ));
    }

    #[test]
    fn test_tada_emoji() {
        let html = "<p>:tada: Congrats!</p>";
        let result = process_jemoji(html);
        assert!(result.contains(
            "src=\"https://github.githubassets.com/images/icons/emoji/unicode/1f389.png\""
        ));
    }

    #[test]
    fn test_plus_one_emoji() {
        let html = "<p>:+1: approved</p>";
        let result = process_jemoji(html);
        assert!(result.contains(
            "src=\"https://github.githubassets.com/images/icons/emoji/unicode/1f44d.png\""
        ));
        assert!(result.contains("title=\":+1:\""));
    }

    #[test]
    fn test_multiple_emoji_in_one_line() {
        let html = "<p>:heart: :tada:</p>";
        let result = process_jemoji(html);
        assert!(
            result.contains("title=\":heart:\""),
            "heart should be replaced"
        );
        assert!(
            result.contains("title=\":tada:\""),
            "tada should be replaced"
        );
    }

    #[test]
    fn test_unknown_shortcode_left_as_is() {
        let html = "<p>:not_a_real_emoji_xyz: stays</p>";
        let result = process_jemoji(html);
        assert_eq!(result, html, "Unknown shortcodes must not be replaced");
    }

    // --- Protected context tests ---

    #[test]
    fn test_emoji_inside_code_not_replaced() {
        let html = "<p>Use <code>:heart:</code> in your config</p>";
        let result = process_jemoji(html);
        assert_eq!(result, html, "Emoji inside <code> must not be converted");
    }

    #[test]
    fn test_emoji_inside_pre_not_replaced() {
        let html = "<pre>:heart: should stay</pre>";
        let result = process_jemoji(html);
        assert_eq!(result, html, "Emoji inside <pre> must not be converted");
    }

    #[test]
    fn test_emoji_inside_tt_not_replaced() {
        let html = "<tt>:heart:</tt>";
        let result = process_jemoji(html);
        assert_eq!(result, html, "Emoji inside <tt> must not be converted");
    }

    #[test]
    fn test_emoji_inside_script_not_replaced() {
        let html = "<script>var x = \":heart:\";</script>";
        let result = process_jemoji(html);
        assert_eq!(result, html, "Emoji inside <script> must not be converted");
    }

    #[test]
    fn test_emoji_inside_style_not_replaced() {
        let html = "<style>/* :heart: */</style>";
        let result = process_jemoji(html);
        assert_eq!(result, html, "Emoji inside <style> must not be converted");
    }

    #[test]
    fn test_emoji_in_html_attribute_not_replaced() {
        let html = "<div title=\":heart:\">content</div>";
        let result = process_jemoji(html);
        assert_eq!(
            result, html,
            "Emoji in HTML attributes must not be converted"
        );
    }

    #[test]
    fn test_emoji_after_code_block_is_replaced() {
        let html = "<p><code>code</code> :heart:</p>";
        let result = process_jemoji(html);
        assert!(
            result.contains("class=\"emoji\""),
            "Emoji after closing code tag should be converted"
        );
        assert!(
            result.contains("<code>code</code>"),
            "Code block content should be preserved"
        );
    }

    #[test]
    fn test_nested_code_in_pre() {
        let html = "<pre><code>:heart:</code></pre>";
        let result = process_jemoji(html);
        assert_eq!(
            result, html,
            "Emoji in nested code/pre must not be converted"
        );
    }

    // --- Edge cases ---

    #[test]
    fn test_double_colon_not_emoji() {
        let html = "<p>:: is not an emoji</p>";
        let result = process_jemoji(html);
        assert_eq!(result, html, ":: should not be treated as emoji");
    }

    #[test]
    fn test_single_colon_not_emoji() {
        let html = "<p>time: 12:00</p>";
        let result = process_jemoji(html);
        assert_eq!(result, html, "Single colons should not be treated as emoji");
    }

    // --- Unicode context tests ---

    #[test]
    fn test_emoji_adjacent_to_unicode_text() {
        let html = "<p>\u{4f60}\u{597d} :heart: \u{4e16}\u{754c}</p>";
        let result = process_jemoji(html);
        assert!(
            result.contains("class=\"emoji\""),
            "Emoji should be replaced in Unicode context"
        );
        assert!(
            result.contains("\u{4f60}\u{597d}"),
            "CJK text should be preserved"
        );
        assert!(
            result.contains("\u{4e16}\u{754c}"),
            "CJK text should be preserved"
        );
    }

    #[test]
    fn test_emoji_with_existing_unicode_emoji() {
        let html = "<p>\u{2764}\u{fe0f} and :heart:</p>";
        let result = process_jemoji(html);
        assert!(
            result.contains("class=\"emoji\""),
            "Shortcode should be replaced"
        );
        assert!(
            result.contains("\u{2764}\u{fe0f}"),
            "Existing unicode emoji should be preserved"
        );
    }

    // --- Integration test: apply_jemoji_if_enabled ---

    #[test]
    fn test_apply_jemoji_disabled() {
        let config = make_config_with_plugins(vec!["jekyll-feed"]);
        let html = "<p>:heart: should not be converted</p>";
        let result = apply_jemoji_if_enabled(html, &config);
        assert_eq!(
            result, html,
            "Jemoji should not process when plugin is not enabled"
        );
    }

    #[test]
    fn test_apply_jemoji_enabled() {
        let config = make_config_with_plugins(vec!["jemoji"]);
        let html = "<p>:heart: should be converted</p>";
        let result = apply_jemoji_if_enabled(html, &config);
        assert!(
            result.contains("class=\"emoji\""),
            "Jemoji should process when plugin is enabled"
        );
    }

    // --- All 20 jekyll-docs emoji ---

    #[test]
    fn test_all_jekyll_docs_emoji() {
        let cases = vec![
            (":+1:", "1f44d"),
            (":bouquet:", "1f490"),
            (":bow:", "1f647"),
            (":broken_heart:", "1f494"),
            (":bug:", "1f41b"),
            (":christmas_tree:", "1f384"),
            (":confetti_ball:", "1f38a"),
            (":gem:", "1f48e"),
            (":heart:", "2764"),
            (":pray:", "1f64f"),
            (":slightly_smiling_face:", "1f642"),
            (":smile:", "1f604"),
            (":smiley:", "1f603"),
            (":sparkles:", "2728"),
            (":sweat_smile:", "1f605"),
            (":tada:", "1f389"),
            (":wave:", "1f44b"),
            (":white_check_mark:", "2705"),
            (":wink:", "1f609"),
            (":x:", "274c"),
        ];
        for (shortcode, expected_codepoint) in cases {
            let html = format!("<p>{}</p>", shortcode);
            let result = process_jemoji(&html);
            let expected_src = format!(
                "https://github.githubassets.com/images/icons/emoji/unicode/{}.png",
                expected_codepoint
            );
            assert!(
                result.contains(&expected_src),
                "Shortcode {} should map to codepoint {}, got: {}",
                shortcode,
                expected_codepoint,
                result
            );
        }
    }

    // --- Table cell emoji (security page pattern) ---

    #[test]
    fn test_emoji_in_table_cell() {
        let html = "<table><tr><td>:white_check_mark:</td><td>:x:</td></tr></table>";
        let result = process_jemoji(html);
        assert!(
            result.contains("class=\"emoji\""),
            "Emoji in table cells should be converted"
        );
        assert!(
            result.contains("2705.png"),
            "white_check_mark should be converted"
        );
        assert!(result.contains("274c.png"), "x should be converted");
    }
}
