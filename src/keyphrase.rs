use crate::crypto::{gen_random_bytes, sha256_first_byte};
use crate::error::ErrorKind;
use crate::keyphrase_type::KeyPhraseType;
use crate::language::{Language, WordList, WordMap};
use crate::util::{checksum, BitWriter, Bits11, IterExt};
use failure::Error;
use std::fmt;

/// Human readable backup phrases which contain most of the information needed to recreate your [EARTH](https://www.earth.engineering) addresses.
///
/// To create a *new* [`KeyPhrase`][KeyPhrase] from a randomly generated key, call [`KeyPhrase::new()`][KeyPhrase::new()].
///
/// To get a [`KeyPhrase`][KeyPhrase] instance for an existing keyphrase, including
/// those generated by other software or hardware wallets, use [`KeyPhrase::from_phrase()`][KeyPhrase::from_phrase()].
///
/// You can get the HD wallet [`Seed`][Seed] from a [`KeyPhrase`][KeyPhrase] by calling [`Seed::new()`][Seed::new()].
/// From there you can either get the raw byte value with [`Seed::as_bytes()`][Seed::as_bytes()], or the hex
/// representation using Rust formatting: `format!("{:X}", seed)`.
///
/// You can also get the original entropy value back from a [`KeyPhrase`][KeyPhrase] with [`KeyPhrase::entropy()`][KeyPhrase::entropy()],
/// but beware that the entropy value is **not the same thing** as an HD wallet seed, and should
/// *never* be used that way.
///
/// [KeyPhrase]: ./keyphrase/struct.KeyPhrase.html
/// [KeyPhrase::new()]: ./keyphrase/struct.KeyPhrase.html#method.new
/// [KeyPhrase::from_phrase()]: ./keyphrase/struct.KeyPhrase.html#method.from_phrase
/// [KeyPhrase::entropy()]: ./keyphrase/struct.KeyPhrase.html#method.entropy
/// [Seed]: ./seed/struct.Seed.html
/// [Seed::new()]: ./seed/struct.Seed.html#method.new
/// [Seed::as_bytes()]: ./seed/struct.Seed.html#method.as_bytes
///
#[derive(Clone)]
pub struct KeyPhrase {
    phrase: String,
    lang: Language,
    entropy: Vec<u8>,
}

impl KeyPhrase {
    /// Generates a new [`KeyPhrase`][KeyPhrase]
    ///
    /// Use [`KeyPhrase::phrase()`][KeyPhrase::phrase()] to get an `str` slice of the generated phrase.
    ///
    /// # Example
    ///
    /// ```
    /// use keyphrase::{KeyPhrase, KeyPhraseType, Language};
    ///
    /// let keyphrase = KeyPhrase::new(KeyPhraseType::Words12, Language::English);
    /// let phrase = keyphrase.phrase();
    ///
    /// println!("phrase: {}", phrase);
    ///
    /// assert_eq!(phrase.split(" ").count(), 12);
    /// ```
    ///
    /// [KeyPhrase]: ./keyphrase/struct.KeyPhrase.html
    /// [KeyPhrase::phrase()]: ./keyphrase/struct.KeyPhrase.html#method.phrase
    pub fn new(keyphrase_type: KeyPhraseType, lang: Language) -> KeyPhrase {
        let entropy: Vec<u8> = gen_random_bytes(keyphrase_type.entropy_bits() / 8);

        KeyPhrase::from_entropy_unchecked(entropy, lang)
    }

    /// Create a [`KeyPhrase`][KeyPhrase] from pre-generated entropy
    ///
    /// # Example
    ///
    /// ```
    /// use keyphrase::{KeyPhrase, KeyPhraseType, Language};
    ///
    /// let entropy = &[0x33, 0xE4, 0x6B, 0xB1, 0x3A, 0x74, 0x6E, 0xA4, 0x1C, 0xDD, 0xE4, 0x5C, 0x90, 0x84, 0x6A, 0x79];
    /// let keyphrase = KeyPhrase::from_entropy(entropy, Language::English).unwrap();
    ///
    /// assert_eq!("crop cash unable insane eight faith inflict route frame loud box vibrant", keyphrase.phrase());
    /// assert_eq!("33E46BB13A746EA41CDDE45C90846A79", format!("{:X}", keyphrase));
    /// ```
    ///
    /// [KeyPhrase]: ../keyphrase/struct.KeyPhrase.html
    pub fn from_entropy(entropy: &[u8], lang: Language) -> Result<KeyPhrase, Error> {
        // Validate entropy size
        KeyPhraseType::for_key_size(entropy.len() * 8)?;

        Ok(Self::from_entropy_unchecked(entropy, lang))
    }

    fn from_entropy_unchecked<E>(entropy: E, lang: Language) -> KeyPhrase
    where
        E: Into<Vec<u8>>,
    {
        let entropy: Vec<u8> = entropy.into();
        let wordlist: &WordList = lang.wordlist();

        let checksum_byte: u8 = sha256_first_byte(&entropy);

        // First, create a byte iterator for the given entropy and the first byte of the
        // hash of the entropy that will serve as the checksum (up to 8 bits for biggest
        // entropy source).
        //
        // Then we transform that into a bits iterator that returns 11 bits at a
        // time (as u16), which we can map to the words on the `wordlist`.
        //
        // Given the entropy is of correct size, this ought to give us the correct word
        // count.
        let phrase: String = entropy
            .iter()
            .chain(Some(&checksum_byte))
            .bits()
            .map(|bits: Bits11| wordlist.get_word(bits))
            .join(" ");

        KeyPhrase {
            phrase,
            lang,
            entropy,
        }
    }

    /// Create a [`KeyPhrase`][KeyPhrase] from an existing keyphrase
    ///
    /// The phrase supplied will be checked for word length and validated according to the checksum
    /// specified in the [KeyPhrase Spec](https://github.com/EarthEngineering/keyphrase/wiki/KeyPhrase-Specification)
    ///
    /// # Example
    ///
    /// ```
    /// use keyphrase::{KeyPhrase, Language};
    ///
    /// let phrase = "park remain person kitchen mule spell knee armed position rail grid ankle";
    /// let keyphrase = KeyPhrase::from_phrase(phrase, Language::English).unwrap();
    ///
    /// assert_eq!(phrase, keyphrase.phrase());
    /// ```
    ///
    /// [KeyPhrase]: ../keyphrase/struct.KeyPhrase.html
    pub fn from_phrase<S>(phrase: S, lang: Language) -> Result<KeyPhrase, Error>
    where
        S: Into<String>,
    {
        let phrase: String = phrase.into();

        // this also validates the checksum and phrase length before returning the entropy so we
        // can store it. We don't use the validate function here to avoid having a public API that
        // takes a phrase string and returns the entropy directly.
        let entropy: Vec<u8> = KeyPhrase::phrase_to_entropy(&phrase, lang)?;

        let keyphrase: KeyPhrase = KeyPhrase {
            phrase,
            lang,
            entropy,
        };

        Ok(keyphrase)
    }

    /// Validate a keyphrase
    ///
    /// The phrase supplied will be checked for word length and validated according to the checksum
    /// specified in the [KeyPhrase Spec](https://github.com/EarthEngineering/keyphrase/wiki/KeyPhrase-Specification).
    ///
    /// # Example
    ///
    /// ```
    /// use keyphrase::{KeyPhrase, Language};
    ///
    /// let test_keyphrase = "park remain person kitchen mule spell knee armed position rail grid ankle";
    ///
    /// assert!(KeyPhrase::validate(test_keyphrase, Language::English).is_ok());
    /// ```
    pub fn validate(phrase: &str, lang: Language) -> Result<(), Error> {
        KeyPhrase::phrase_to_entropy(phrase, lang)?;

        Ok(())
    }

    /// Calculate the checksum, verify it and return the entropy
    ///
    /// Only intended for internal use, as returning a `Vec<u8>` that looks a bit like it could be
    /// used as the seed is likely to cause problems for someone eventually. All the other functions
    /// that return something like that are explicit about what it is and what to use it for.
    fn phrase_to_entropy(phrase: &str, lang: Language) -> Result<Vec<u8>, Error> {
        let wordmap: &WordMap = lang.wordmap();

        // Preallocate enough space for the longest possible word list
        let mut bits = BitWriter::with_capacity(264);

        for word in phrase.split(" ") {
            bits.push(wordmap.get_bits(&word)?);
        }

        let mtype: KeyPhraseType = KeyPhraseType::for_word_count(bits.len() / 11)?;

        debug_assert!(
            bits.len() == mtype.total_bits(),
            "Insufficient amount of bits to validate"
        );

        let mut entropy = bits.into_bytes();
        let entropy_bytes: usize = mtype.entropy_bits() / 8;

        let actual_checksum: u8 = checksum(entropy[entropy_bytes], mtype.checksum_bits());

        // Truncate to get rid of the byte containing the checksum
        entropy.truncate(entropy_bytes);

        let checksum_byte: u8 = sha256_first_byte(&entropy);
        let expected_checksum: u8 = checksum(checksum_byte, mtype.checksum_bits());

        if actual_checksum != expected_checksum {
            Err(ErrorKind::InvalidChecksum)?;
        }

        Ok(entropy)
    }

    /// Get the keyphrase as a string reference.
    ///
    /// # Example
    ///
    /// ```
    /// use keyphrase::{KeyPhrase, KeyPhraseType, Language};
    ///
    /// let keyphrase = KeyPhrase::new(KeyPhraseType::Words12, Language::English);
    ///
    /// let phrase = keyphrase.phrase();
    /// ```
    pub fn phrase(&self) -> &str {
        &self.phrase
    }

    /// Consume the `KeyPhrase` and return the phrase as a `String`.
    ///
    /// This operation doesn't perform any allocations.
    ///
    /// # Example
    ///
    /// ```
    /// use keyphrase::{KeyPhrase, KeyPhraseType, Language};
    ///
    /// let keyphrase = KeyPhrase::new(KeyPhraseType::Words12, Language::English);
    ///
    /// let phrase = keyphrase.into_phrase();
    /// ```
    pub fn into_phrase(self) -> String {
        self.phrase
    }

    /// Get the original entropy value of the keyphrase as a slice.
    ///
    /// # Example
    ///
    /// ```
    /// use keyphrase::{KeyPhrase, Language};
    ///
    /// let phrase = "park remain person kitchen mule spell knee armed position rail grid ankle";
    ///
    /// let keyphrase = KeyPhrase::from_phrase(phrase, Language::English).unwrap();
    ///
    /// let entropy: &[u8] = keyphrase.entropy();
    /// ```
    ///
    /// **Note:** You shouldn't use the generated entropy as secrets, for that generate a new
    /// `Seed` from the `KeyPhrase`.
    pub fn entropy(&self) -> &[u8] {
        &self.entropy
    }

    /// Get the [`Language`][Language]
    ///
    /// [Language]: ../language/struct.Language.html
    ///
    /// # Example
    ///
    /// ```
    /// use keyphrase::{KeyPhrase, Language};
    ///
    /// let phrase = "park remain person kitchen mule spell knee armed position rail grid ankle";
    ///
    /// let keyphrase = KeyPhrase::from_phrase(phrase, Language::English).unwrap();
    ///
    /// let lang = keyphrase.language();
    /// ```
    pub fn language(&self) -> Language {
        self.lang
    }
}

impl AsRef<str> for KeyPhrase {
    fn as_ref(&self) -> &str {
        self.phrase()
    }
}

impl fmt::Display for KeyPhrase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.phrase(), f)
    }
}

impl fmt::Debug for KeyPhrase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.phrase(), f)
    }
}

impl fmt::LowerHex for KeyPhrase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            f.write_str("0x")?;
        }

        for byte in self.entropy() {
            write!(f, "{:x}", byte)?;
        }

        Ok(())
    }
}

impl fmt::UpperHex for KeyPhrase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            f.write_str("0x")?;
        }

        for byte in self.entropy() {
            write!(f, "{:X}", byte)?;
        }

        Ok(())
    }
}

impl From<KeyPhrase> for String {
    fn from(val: KeyPhrase) -> String {
        val.into_phrase()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn back_to_back() {
        let m1: KeyPhrase = KeyPhrase::new(KeyPhraseType::Words12, Language::English);
        let m2: KeyPhrase = KeyPhrase::from_phrase(m1.phrase(), Language::English).unwrap();
        let m3: KeyPhrase = KeyPhrase::from_entropy(m1.entropy(), Language::English).unwrap();

        assert_eq!(m1.entropy(), m2.entropy(), "Entropy must be the same");
        assert_eq!(m1.entropy(), m3.entropy(), "Entropy must be the same");
        assert_eq!(m1.phrase(), m2.phrase(), "Phrase must be the same");
        assert_eq!(m1.phrase(), m3.phrase(), "Phrase must be the same");
    }

    #[test]
    fn keyphrase_from_entropy() {
        let entropy: &[u8; 16] = &[
            0x33, 0xE4, 0x6B, 0xB1, 0x3A, 0x74, 0x6E, 0xA4, 0x1C, 0xDD, 0xE4, 0x5C, 0x90, 0x84,
            0x6A, 0x79,
        ];
        let phrase: &str =
            "crop cash unable insane eight faith inflict route frame loud box vibrant";

        let keyphrase: KeyPhrase = KeyPhrase::from_entropy(entropy, Language::English).unwrap();

        assert_eq!(phrase, keyphrase.phrase());
    }

    #[test]
    fn keyphrase_from_phrase() {
        let entropy: &[u8; 16] = &[
            0x33, 0xE4, 0x6B, 0xB1, 0x3A, 0x74, 0x6E, 0xA4, 0x1C, 0xDD, 0xE4, 0x5C, 0x90, 0x84,
            0x6A, 0x79,
        ];
        let phrase: &str =
            "crop cash unable insane eight faith inflict route frame loud box vibrant";

        let keyphrase: KeyPhrase = KeyPhrase::from_phrase(phrase, Language::English).unwrap();

        assert_eq!(entropy, keyphrase.entropy());
    }

    #[test]
    fn keyphrase_format() {
        let keyphrase: KeyPhrase = KeyPhrase::new(KeyPhraseType::Words15, Language::English);

        assert_eq!(keyphrase.phrase(), format!("{}", keyphrase));
    }

    #[test]
    fn keyphrase_hex_format() {
        let entropy: &[u8; 16] = &[
            0x33, 0xE4, 0x6B, 0xB1, 0x3A, 0x74, 0x6E, 0xA4, 0x1C, 0xDD, 0xE4, 0x5C, 0x90, 0x84,
            0x6A, 0x79,
        ];

        let keyphrase: KeyPhrase = KeyPhrase::from_entropy(entropy, Language::English).unwrap();

        assert_eq!(
            format!("{:x}", keyphrase),
            "33e46bb13a746ea41cdde45c90846a79"
        );
        assert_eq!(
            format!("{:X}", keyphrase),
            "33E46BB13A746EA41CDDE45C90846A79"
        );
        assert_eq!(
            format!("{:#x}", keyphrase),
            "0x33e46bb13a746ea41cdde45c90846a79"
        );
        assert_eq!(
            format!("{:#X}", keyphrase),
            "0x33E46BB13A746EA41CDDE45C90846A79"
        );
    }
}
