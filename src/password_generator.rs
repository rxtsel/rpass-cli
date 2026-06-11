use rand::Rng;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;

const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const NUMBERS: &str = "0123456789";
const DEFAULT_SYMBOLS: &str = "!@#$%^&*_-+=";
const DEFAULT_PASSWORD_LENGTH: usize = 14;
const DEFAULT_PASSPHRASE_WORDS: usize = 4;
const DEFAULT_PASSPHRASE_SEPARATOR: &str = "-";
const MAX_PASSWORD_LENGTH: usize = 1024;
const MAX_PASSPHRASE_WORDS: usize = 20;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PasswordOptions {
    pub length: usize,
    pub lowercase: bool,
    pub uppercase: bool,
    pub numbers: bool,
    pub symbols: Option<String>,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: DEFAULT_PASSWORD_LENGTH,
            lowercase: true,
            uppercase: true,
            numbers: true,
            symbols: Some(DEFAULT_SYMBOLS.to_string()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PassphraseOptions {
    pub words: usize,
    pub separator: String,
    pub capitalize: bool,
    pub number: bool,
}

impl Default for PassphraseOptions {
    fn default() -> Self {
        Self {
            words: DEFAULT_PASSPHRASE_WORDS,
            separator: DEFAULT_PASSPHRASE_SEPARATOR.to_string(),
            capitalize: false,
            number: false,
        }
    }
}

pub fn default_password_length() -> usize {
    DEFAULT_PASSWORD_LENGTH
}

pub fn default_passphrase_words() -> usize {
    DEFAULT_PASSPHRASE_WORDS
}

pub fn default_passphrase_separator() -> &'static str {
    DEFAULT_PASSPHRASE_SEPARATOR
}

pub fn max_password_length() -> usize {
    MAX_PASSWORD_LENGTH
}

pub fn max_passphrase_words() -> usize {
    MAX_PASSPHRASE_WORDS
}

pub fn generate_password(options: &PasswordOptions) -> Result<String, PasswordGeneratorError> {
    validate_password_options(options)?;

    let character_sets = enabled_character_sets(options);
    let all_characters = character_sets.iter().flatten().copied().collect::<Vec<_>>();
    let mut rng = OsRng;
    let mut password = Vec::with_capacity(options.length);

    for set in &character_sets {
        password.push(*set.choose(&mut rng).expect("validated character set"));
    }

    while password.len() < options.length {
        password.push(
            *all_characters
                .choose(&mut rng)
                .expect("validated character set"),
        );
    }

    password.shuffle(&mut rng);
    Ok(password.into_iter().collect())
}

pub fn generate_passphrase(options: &PassphraseOptions) -> Result<String, PasswordGeneratorError> {
    validate_passphrase_options(options)?;

    let mut rng = OsRng;
    let mut parts = (0..options.words)
        .map(|_| {
            let word = memorable_wordlist::WORDS
                .choose(&mut rng)
                .expect("word list is not empty");

            if options.capitalize {
                capitalize_word(word)
            } else {
                (*word).to_string()
            }
        })
        .collect::<Vec<_>>();

    if options.number {
        parts.push(rng.gen_range(0..10).to_string());
    }

    Ok(parts.join(&options.separator))
}

fn validate_password_options(options: &PasswordOptions) -> Result<(), PasswordGeneratorError> {
    if options.length == 0 || options.length > MAX_PASSWORD_LENGTH {
        return Err(PasswordGeneratorError::InvalidLength {
            min: 1,
            max: MAX_PASSWORD_LENGTH,
        });
    }

    let character_sets = enabled_character_sets(options);

    if character_sets.is_empty() {
        return Err(PasswordGeneratorError::EmptyCharacterSet);
    }

    if options.length < character_sets.len() {
        return Err(PasswordGeneratorError::LengthTooShortForRequiredSets {
            length: options.length,
            required_sets: character_sets.len(),
        });
    }

    Ok(())
}

fn validate_passphrase_options(options: &PassphraseOptions) -> Result<(), PasswordGeneratorError> {
    if options.words == 0 || options.words > MAX_PASSPHRASE_WORDS {
        return Err(PasswordGeneratorError::InvalidWordCount {
            min: 1,
            max: MAX_PASSPHRASE_WORDS,
        });
    }

    if options.separator.is_empty() {
        return Err(PasswordGeneratorError::EmptySeparator);
    }

    Ok(())
}

fn enabled_character_sets(options: &PasswordOptions) -> Vec<Vec<char>> {
    let mut sets = Vec::new();

    if options.lowercase {
        sets.push(LOWERCASE.chars().collect());
    }

    if options.uppercase {
        sets.push(UPPERCASE.chars().collect());
    }

    if options.numbers {
        sets.push(NUMBERS.chars().collect());
    }

    if let Some(symbols) = &options.symbols {
        let symbols = symbols.chars().collect::<Vec<_>>();

        if !symbols.is_empty() {
            sets.push(symbols);
        }
    }

    sets
}

fn capitalize_word(word: &str) -> String {
    let mut chars = word.chars();

    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PasswordGeneratorError {
    #[error("password length must be between {min} and {max}")]
    InvalidLength { min: usize, max: usize },

    #[error("at least one character set must be enabled")]
    EmptyCharacterSet,

    #[error("password length {length} is too short for {required_sets} required character sets")]
    LengthTooShortForRequiredSets { length: usize, required_sets: usize },

    #[error("passphrase word count must be between {min} and {max}")]
    InvalidWordCount { min: usize, max: usize },

    #[error("passphrase separator cannot be empty")]
    EmptySeparator,
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_PASSWORD_LENGTH, PassphraseOptions, PasswordOptions, generate_passphrase,
        generate_password,
    };

    #[test]
    fn default_password_has_four_character_classes() {
        let password = generate_password(&PasswordOptions::default()).expect("password");

        assert_eq!(password.chars().count(), DEFAULT_PASSWORD_LENGTH);
        assert!(
            password
                .chars()
                .any(|character| character.is_ascii_lowercase())
        );
        assert!(
            password
                .chars()
                .any(|character| character.is_ascii_uppercase())
        );
        assert!(password.chars().any(|character| character.is_ascii_digit()));
        assert!(
            password
                .chars()
                .any(|character| !character.is_alphanumeric())
        );
    }

    #[test]
    fn custom_symbols_limit_symbol_set() {
        let password = generate_password(&PasswordOptions {
            length: 32,
            symbols: Some("_".to_string()),
            ..PasswordOptions::default()
        })
        .expect("password");

        assert!(password.chars().any(|character| character == '_'));
        assert!(
            password
                .chars()
                .filter(|character| !character.is_alphanumeric())
                .all(|character| character == '_')
        );
    }

    #[test]
    fn rejects_length_shorter_than_required_sets() {
        let error = generate_password(&PasswordOptions {
            length: 3,
            ..PasswordOptions::default()
        })
        .unwrap_err();

        assert!(matches!(
            error,
            super::PasswordGeneratorError::LengthTooShortForRequiredSets { .. }
        ));
    }

    #[test]
    fn passphrase_uses_requested_separator() {
        let passphrase = generate_passphrase(&PassphraseOptions {
            words: 4,
            separator: "-".to_string(),
            capitalize: false,
            number: false,
        })
        .expect("passphrase");

        assert_eq!(passphrase.split('-').count(), 4);
    }

    #[test]
    fn passphrase_can_append_number() {
        let passphrase = generate_passphrase(&PassphraseOptions {
            words: 4,
            separator: "-".to_string(),
            capitalize: true,
            number: true,
        })
        .expect("passphrase");
        let parts = passphrase.split('-').collect::<Vec<_>>();

        assert_eq!(parts.len(), 5);
        assert!(
            parts[..4]
                .iter()
                .all(|word| word.chars().next().is_some_and(char::is_uppercase))
        );
        assert!(parts[4].chars().all(|character| character.is_ascii_digit()));
    }
}
