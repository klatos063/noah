use crate::errors::ZeiError;
use crate::serialization::ZeiFromToBytes;
use rand::CryptoRng;
use rand::Rng;

use crate::errors::ZeiError::SignatureError;
use curve25519_dalek::edwards::CompressedEdwardsY;
use curve25519_dalek::edwards::EdwardsPoint;
use curve25519_dalek::scalar::Scalar;
use ed25519_dalek::PublicKey;
use ed25519_dalek::SecretKey;
use ed25519_dalek::Signature;
use crate::algebra::pairing::Pairing;
use crate::algebra::groups::{Scalar as ScalatTrait, Group};
use digest::Digest;

pub const XFR_SECRET_KEY_LENGTH: usize = ed25519_dalek::SECRET_KEY_LENGTH;
//pub const XFR_PUBLIC_KEY_LENGTH: usize = ed25519_dalek::PUBLIC_KEY_LENGTH;

pub const KEY_BASE_POINT: CompressedEdwardsY =
  curve25519_dalek::constants::ED25519_BASEPOINT_COMPRESSED;

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub struct XfrPublicKey(pub(crate) PublicKey);
#[derive(Default, Debug)]
pub struct XfrSecretKey(pub(crate) SecretKey);
#[derive(Default, Debug)]
pub struct XfrKeyPair {
  public: XfrPublicKey,
  secret: XfrSecretKey,
}

type HashFnc = sha2::Sha512;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XfrSignature(pub Signature);

impl XfrPublicKey {
  pub fn get_curve_point(&self) -> Result<EdwardsPoint, ZeiError> {
    CompressedEdwardsY::from_slice(self.zei_to_bytes().as_slice()).decompress().
            ok_or(ZeiError::DecompressElementError)
  }

  pub fn verify(&self, message: &[u8], signature: &XfrSignature) -> Result<(), ZeiError> {
    Ok(self.0.verify::<HashFnc>(message, &signature.0)?)
  }

  pub fn as_bytes(&self) -> &[u8] {
    self.0.as_bytes()
  }
}

impl ZeiFromToBytes for XfrPublicKey {
  fn zei_to_bytes(&self) -> Vec<u8> {
    let bytes = self.0.to_bytes();
    let mut vec = vec![];
    vec.extend_from_slice(&bytes[..]);
    vec
  }

  fn zei_from_bytes(bytes: &[u8]) -> Self {
    XfrPublicKey(PublicKey::from_bytes(bytes).unwrap())
  }
}

impl XfrSecretKey {
  pub fn sign(&self, message: &[u8], public_key: &XfrPublicKey) -> XfrSignature {
    let expanded = self.0.expand::<HashFnc>();
    let sign = expanded.sign::<HashFnc>(message, &public_key.0);

    XfrSignature(sign)
  }

  pub fn as_scalar_multiply_by_curve_point(&self, y: &EdwardsPoint) -> EdwardsPoint {
    let expanded = self.0.expand::<HashFnc>();
    //expanded.key is not public, I need to extract it via serialization
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&expanded.to_bytes()[0..32]); //1st 32 bytes are key
    let key_scalar = Scalar::from_bits(key_bytes);
    key_scalar * y
  }

  fn clone(&self) -> Self {
    let bytes = self.zei_to_bytes();
    XfrSecretKey::zei_from_bytes(bytes.as_slice())
  }
}

impl ZeiFromToBytes for XfrSecretKey {
  fn zei_to_bytes(&self) -> Vec<u8> {
    let bytes = self.0.to_bytes();
    let mut vec = vec![];
    vec.extend_from_slice(&bytes[..]);
    vec
  }

  fn zei_from_bytes(bytes: &[u8]) -> Self {
    XfrSecretKey(SecretKey::from_bytes(bytes).unwrap())
  }
}

impl XfrKeyPair {
  pub fn generate<R: CryptoRng + Rng>(prng: &mut R) -> Self
    where R: CryptoRng + Rng
  {
    let kp = ed25519_dalek::Keypair::generate::<HashFnc, _>(prng);
    XfrKeyPair { public: XfrPublicKey(kp.public),
                 secret: XfrSecretKey(kp.secret) }
  }

  pub fn get_pk_ref(&self) -> &XfrPublicKey {
    &self.public
  }

  pub fn get_sk_ref(&self) -> &XfrSecretKey {
    &self.secret
  }

  pub fn get_sk(&self) -> XfrSecretKey {
    self.secret.clone()
  }

  pub fn sign(&self, msg: &[u8]) -> XfrSignature {
    self.secret.sign(msg, &self.public)
  }
}

impl ZeiFromToBytes for XfrKeyPair {
  fn zei_to_bytes(&self) -> Vec<u8> {
    let mut vec = vec![];
    vec.extend_from_slice(self.secret.zei_to_bytes().as_slice());
    vec.extend_from_slice(self.public.zei_to_bytes().as_slice());
    vec
  }

  fn zei_from_bytes(bytes: &[u8]) -> Self {
    XfrKeyPair { secret: XfrSecretKey::zei_from_bytes(&bytes[0..XFR_SECRET_KEY_LENGTH]),
                 public: XfrPublicKey::zei_from_bytes(&bytes[XFR_SECRET_KEY_LENGTH..]) }
  }
}

////Primitive for multisignatures /////
///A multisignature is defined as a signature on a message that must verify against a list of public keys instead of one
// naive implementation below
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct XfrMultiSig {
  pub(crate) signatures: Vec<XfrSignature>,
}

pub fn verify_multisig(keylist: &[XfrPublicKey],
                       message: &[u8],
                       multi_signature: &XfrMultiSig)
                       -> Result<(), ZeiError> {
  if multi_signature.signatures.len() != keylist.len() {
    return Err(SignatureError); //TODO return MultiSignatureError differnet length
  }
  for (pk, signature) in keylist.iter().zip(multi_signature.signatures.iter()) {
    pk.verify(message, signature)?; //TODO return MultiSignatureError
  }
  Ok(())
}

pub fn sign_multisig(keylist: &[XfrKeyPair], message: &[u8]) -> XfrMultiSig {
  let mut signatures = vec![];
  for keypair in keylist.iter() {
    let signature = keypair.sign(message);
    signatures.push(signature);
  }
  XfrMultiSig { signatures }
}

pub fn bls_sign<S: ScalatTrait, P: Pairing<S>>(key: &S, message: &[u8]) -> P::G2{
  let mut hash = HashFnc::default();
  hash.input(message);
  let hashed = P::G2::from_hash(hash);
  P::g2_mul_scalar(&hashed, key)
}


pub fn bls_verify<S: ScalatTrait, P: Pairing<S>>(key: &P::G1, message: &[u8], signature: &P::G2) -> Result<(), ZeiError>{
  let mut hash = HashFnc::default();
  hash.input(message);
  let hashed = P::G2::from_hash(hash);

  let a = P::pairing(&P::G1::get_base(), signature);
  let b = P::pairing(key, &hashed);

  match a == b{
    true => Ok(()),
    false => Err(ZeiError::SignatureError)
  }
}

/*

pub fn bls_aggregate();

pub fn bls_verify_aggregated();

*/

#[cfg(test)]
mod test {
  use crate::basic_crypto::signatures::{sign_multisig, verify_multisig, XfrKeyPair, XfrPublicKey};
  use crate::errors::ZeiError::SignatureError;
  use rand::SeedableRng;
  use rand_chacha::ChaChaRng;
  use crate::algebra::bls12_381::BLSScalar;

  #[test]
  fn signatures() {
    let mut prng = rand_chacha::ChaChaRng::from_seed([0u8; 32]);

    let keypair = XfrKeyPair::generate(&mut prng);
    let message = "";

    let sig = keypair.sign(message.as_bytes());
    assert_eq!(Ok(()), keypair.get_pk_ref().verify("".as_bytes(), &sig));
    //same test with secret key
    let sig = keypair.get_sk_ref()
                     .sign(message.as_bytes(), keypair.get_pk_ref());
    assert_eq!(Ok(()), keypair.get_pk_ref().verify("".as_bytes(), &sig));

    //test again with fresh same key
    let mut prng = rand_chacha::ChaChaRng::from_seed([0u8; 32]);
    let keypair = XfrKeyPair::generate(&mut prng);
    assert_eq!(Ok(()), keypair.get_pk_ref().verify("".as_bytes(), &sig));

    let keypair = XfrKeyPair::generate(&mut prng);
    let message = [10u8; 500];
    let sig = keypair.sign(&message);
    assert_eq!(Err(SignatureError),
               keypair.get_pk_ref().verify("".as_bytes(), &sig),
               "Verifying sig on different message should have return Err(Signature Error)");
    assert_eq!(Ok(()),
               keypair.get_pk_ref().verify(&message, &sig),
               "Verifying sig on samme message should have return Ok(())");
    //test again with secret key
    let sk = keypair.get_sk_ref();
    let pk = keypair.get_pk_ref();
    let sig = sk.sign(&message, pk);
    assert_eq!(Err(SignatureError),
               keypair.get_pk_ref().verify("".as_bytes(), &sig),
               "Verifying sig on different message should have return Err(Signature Error)");
    assert_eq!(Ok(()),
               pk.verify(&message, &sig),
               "Verifying sig on samme message should have return Ok(())");

    // test with different keys
    let keypair = XfrKeyPair::generate(&mut prng);
    assert_eq!(Err(SignatureError),
               keypair.get_pk_ref().verify(&message, &sig),
               "Verifying sig on with a different key should have return Err(Signature Error)");
  }

  fn generate_keys(prng: &mut ChaChaRng, n: usize) -> Vec<XfrKeyPair> {
    let mut v = vec![];
    for _ in 0..n {
      v.push(XfrKeyPair::generate(prng));
    }
    v
  }

  use crate::algebra::bls12_381::{BLSG1, BLSGt};
  use crate::algebra::groups::{Group, Scalar as ScalarTrait};
  #[test]
  fn bls_signatures(){

    let mut prng = rand_chacha::ChaChaRng::from_seed([1u8; 32]);
    let sk = BLSScalar::random_scalar(&mut prng);
    let pk = BLSG1::get_base().mul(&sk);

    let message = b"this is a message";

    let signature = super::bls_sign::<BLSScalar, BLSGt>(&sk, message);

    assert_eq!(Ok(()), super::bls_verify::<BLSScalar, BLSGt>(&pk, message, &signature));
    assert_eq!(Err(crate::errors::ZeiError::SignatureError), super::bls_verify::<BLSScalar, BLSGt>(&pk, b"wrong message", &signature))
  }

  #[test]
  fn multisig() {
    let mut prng = rand_chacha::ChaChaRng::from_seed([1u8; 32]);
    // test with one key
    let keypairs = generate_keys(&mut prng, 1);
    let pk = keypairs.get(0).unwrap().get_pk_ref();
    let msig = sign_multisig(keypairs.as_slice(), "HELLO".as_bytes());
    assert_eq!(Ok(()),
               verify_multisig(&[pk.clone()], "HELLO".as_bytes(), &msig),
               "Multisignature should have verify correctly");
    //try with more keys
    let extra_key = XfrKeyPair::generate(&mut prng);
    assert_eq!(Err(SignatureError),
               verify_multisig(&[pk.clone(), extra_key.get_pk_ref().clone()],
                               "HELLO".as_bytes(),
                               &msig),
               "Multisignature should have not verify correctly");

    // test with two keys
    let keypairs = generate_keys(&mut prng, 2);
    let pk0 = keypairs.get(0).unwrap().get_pk_ref();
    let pk1 = keypairs.get(1).unwrap().get_pk_ref();
    let msig = sign_multisig(keypairs.as_slice(), "HELLO".as_bytes());
    assert_eq!(Ok(()),
               verify_multisig(&[pk0.clone(), pk1.clone()], "HELLO".as_bytes(), &msig),
               "Multisignature should have verify correctly");

    let newkeypair = XfrKeyPair::generate(&mut prng);
    let pk2 = newkeypair.get_pk_ref();
    assert_eq!(Err(SignatureError),
               verify_multisig(&[pk0.clone(), pk1.clone(), pk2.clone()],
                               "HELLO".as_bytes(),
                               &msig),
               "Message was signed with two keys");
    assert_eq!(Err(SignatureError),
               verify_multisig(&[pk0.clone(), pk2.clone()], "HELLO".as_bytes(), &msig),
               "Message was signed under different key set");

    // test with 20 keys
    let keypairs = generate_keys(&mut prng, 20);
    let pks: Vec<XfrPublicKey> = keypairs.iter().map(|x| x.get_pk_ref().clone()).collect();
    let msig = sign_multisig(keypairs.as_slice(), "HELLO".as_bytes());
    assert_eq!(Ok(()),
               verify_multisig(pks.as_slice(), "HELLO".as_bytes(), &msig),
               "Multisignature should have verify correctly");
  }
}
