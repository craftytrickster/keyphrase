use crate::keyphrase_type::KeyPhraseType;

#[derive(Debug, Fail)]
pub enum ErrorKind {
	#[fail(display = "invalid checksum")]
	InvalidChecksum,
	#[fail(display = "invalid word in phrase")]
	InvalidWord,
	#[fail(display = "invalid keysize: {}", _0)]
	InvalidKeysize(usize),
	#[fail(display = "invalid number of words in phrase: {}", _0)]
	InvalidWordLength(usize),
	#[fail(
		display = "invalid entropy length {}bits for keyphrase type {:?}",
		_0, _1
	)]
	InvalidEntropyLength(usize, KeyPhraseType),
}
