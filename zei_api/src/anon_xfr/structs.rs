use crate::anon_xfr::keys::{AXfrKeyPair, AXfrPubKey};
use crate::xfr::structs::{AssetType, OwnerMemo};
use algebra::bls12_381::{BLSScalar, Bls12381};
use algebra::jubjub::JubjubScalar;
use crypto::basics::hybrid_encryption::XPublicKey;
use poly_iops::commitments::kzg_poly_com::KZGCommitmentScheme;
use poly_iops::plonk::protocol::prover::PlonkPf;

pub type Nullifier = BLSScalar;
pub type Commitment = BLSScalar;
pub type BlindFactor = BLSScalar;

/// A Merkle tree node which consists of the following:
/// * `siblings1` - the 1st sibling of the tree node
/// * `siblings2` - the 2nd sibling of the tree node
/// * `is_left_child` - indicates whether the tree node is the left child of its parent
/// * `is_right_child` - indicates whether the tree node is the right child of its parent
#[derive(Debug, Clone)]
pub struct MTNode {
  pub siblings1: BLSScalar,
  pub siblings2: BLSScalar,
  pub is_left_child: u8,
  pub is_right_child: u8,
}

pub type SnarkProof = PlonkPf<KZGCommitmentScheme<Bls12381>>;

/// Anonymous transfers structure
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct AXfrBody {
  pub inputs: Vec<(Nullifier, AXfrPubKey)>,
  pub outputs: Vec<AnonBlindAssetRecord>,
  pub proof: AXfrProof,
  pub memo: Vec<OwnerMemo>,
}

/// Asset record to be published
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct AnonBlindAssetRecord {
  pub amount_type_commitment: Commitment,
  pub public_key: AXfrPubKey,
}

/// Proof for an AXfrBody correctness
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct AXfrProof {
  pub snark_proof: SnarkProof,
  pub merkle_root: BLSScalar,
}

/// MT PATH, merkle root value, leaf identifier
pub struct MTLeafInfo {
  pub path: MTPath,
  pub root: BLSScalar,
  pub uid: u64,
}

/// Open Asset record for an AnonBlindAssetRecord
pub struct OpenAnonBlindAssetRecord<'a> {
  pub amount: u64,
  pub asset_type: AssetType,
  pub blind: BLSScalar,
  pub key_rand: JubjubScalar,
  pub mt_leaf_info: MTLeafInfo,
  pub key_pair: AXfrKeyPair,
  pub abar: &'a AnonBlindAssetRecord,
}

/// Template describing an output asset record information
pub struct AnonAssetRecordTemplate {
  pub amount: u64,
  pub asset_type: AssetType,
  pub public_key: AXfrPubKey,
  pub encryption_key: XPublicKey,
}

/// An authentication path of a ternary Merkle tree.
#[derive(Debug, Clone)]
pub struct MTPath {
  pub nodes: Vec<MTNode>,
}

impl MTPath {
  pub fn new(nodes: Vec<MTNode>) -> Self {
    Self { nodes }
  }
}
