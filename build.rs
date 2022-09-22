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
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::vec;

use hex::FromHex;
use openssl;

fn generate_pubkey() -> std::io::Result<()> {
    let filename = "pubkey.pem";
    println!("cargo:rerun-if-changed={}", filename);
    let pubkey_pem = fs::read(filename)?;

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

fn generate_blacklist() -> std::io::Result<()> {

    let filename = "blacklist.txt";
    println!("cargo:rerun-if-changed={}", filename);
    let file = File::open(filename)?;
    let reader = BufReader::new(file);

    let mut hash_list: Vec<[u8;32]> = vec![];

    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if !line.starts_with("#") && line.len() > 0 {
            if line.len() < 64 {
                panic!("Blacklist.txt line {}: invalid SHA256 hash: too short!", i);
            }

            //let hex_bytes = hex::decode(line);
            match <[u8;32]>::from_hex(line) {
                Ok(hash) => {
                    hash_list.push(hash);
                }
                Err(_) => {
                    println!("Blacklist.txt line {}: '{}' is not a valid SHA256 hash", i, line);
                }
            }
        }
    }

    let mut out = File::create("src//blacklist.rs")?;
    out.write_all(b"\n//NOTE: this file is auto-generated from `blacklist.txt`, do not edit!\n\n").ok();
    out.write_all(b"// Blacklisted SHA-256 hashes, each 32 bytes (32*8=256 bits):\n").ok();
    write!(&mut out, "pub const FW_BLACKLIST: [[u8; 32]; {}] = [\n", hash_list.len()).ok();

    for hash in hash_list {
        out.write_all(b"\t[").unwrap();
        for byte in hash {
            write!(&mut out, "0x{:02X}, ", byte).ok();
        }
        out.write_all(b"],\n").unwrap();
    }
    out.write_all(b"\n];")?;
    Ok(())
}

fn main() {
    
    generate_pubkey().expect("Failed to build public key: See pubkey.pem.example for an example!");

    generate_blacklist().expect("Failed to build blacklist: See blacklist.txt.example for an example!");
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
