//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use openssl;
//use pem_parser;
//use ecdsa;
//use p256::{PublicKey};
//use p256::ecdsa;
//use signature::Signature;

fn generate_pubkey() -> std::io::Result<()> {
    let pubkey_pem = fs::read("pubkey.pem").unwrap();  
    let ec_key = openssl::ec::EcKey::public_key_from_pem(&pubkey_pem).unwrap();
    let pubkey = ec_key.public_key();
    let grp = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::X9_62_PRIME256V1).unwrap();
    let form = openssl::ec::PointConversionForm::UNCOMPRESSED;
    let mut bignum_ctx = openssl::bn::BigNumContext::new().unwrap();
    let pub_bytes = pubkey.to_bytes(&grp, form, &mut bignum_ctx).unwrap();
    
    
    //let pubkey_der = pem_parser::pem_to_der(&pubkey_pem);
    
    //ecdsa::Asn1Signature::from_bytes(&pubkey_der);



    let mut out = File::create("src//pubkey.rs")?;
    out.write_all(b"\n//NOTE: this file is auto-generated from `pubkey.pem`, do not edit!\n\n").ok();

    out.write_all(b"// EC Public key in raw R,S format.\n").ok();
    out.write_all(b"pub const FW_SIGN_PUBKEY: [u8; 65] = [\n").ok();
    for i in 0..pub_bytes.len() {
        write!(&mut out, "0x{:02X},", pub_bytes[i]).ok();
        if i % 8 == 7 {
            out.write_all(b"\n").ok();
        } else {
            out.write_all(b" ").ok();
        }
        
    }
    out.write_all(b"\n];")?;
    Ok(())
}

fn main() {
    
    generate_pubkey().unwrap();
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");
}
