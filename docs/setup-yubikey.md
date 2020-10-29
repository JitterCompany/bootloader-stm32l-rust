#
# Below are instructions on how to generate a private key and import it
# into a set of yubikeys for (code?) signing.
#
# Note that you may want to configure a PIN+PUK on the yubikey
# for enhanced security, as well as a management key to prevent unauthorized
# modifications. This is however outside of the scope of these instructions.



#
# Create a ECC key, import into a set of yubikeys
#
# NOTE: this MUST BE DONE ON AN AIRGAPPED, STATELESS MACHINE!
# Ideally this is a machine without disk or wifi, booted from a liveCD
# This avoids leaking the private key or (accidentally) leaving it
# (even if you delete it, it may be backed up or otherwise remain on-disk).
#

# Step 1: generate a private key
openssl ecparam -out yubi.key -name prime256v1 -genkey

# Step 2: Creata a public key and CSR
openssl ec -inform PEM -in yubi.key -outform PEM -pubout -out yubi.pub
openssl req -new -key yubi.key -out yubi.csr -sha256 -subj "/CN=cert/"


# Step 3: import private key into each yubikey
# NOTE: It is strongly recommended to have AT LEAST two yubikeys (for backup).
# The yubikeys will be the only place the private key is stored.
# The private key CANNOT BE EXTRACTED OR DUPLICATED later!! 
yubico-piv-tool -aimport-key -s9c --touch-policy=always -iyubi.key

# Step 4: Setup certificates for each yubikey
# Even though the yubikeys already have a private key, most tools will
# expect a matching certificate to be present. So let's self-sign one.
# Note: replace '<some-number>' and '<key name>' with your own values.
# Note: touch the yubikey to complete the command
yubico-piv-tool -a verify -a selfsign --valid-days <some-number> -s 9c -S "/CN=<key name>/" -i yubi.pub -o yubi.crt
yubico-piv-tool -a import-certificate -s 9c -i yubi.crt 

# Step 5: cleanup.
# The private key is now safely stored inside your yubikeys, so it can be deleted.
# Note that most filesystems don't really properly delete data. That is why you MUST use
# a liveCD or similar stateless setup!
rm yubi.key
rm yubi.csr
rm yubi.crt
rm yubi.pub


# DONE! You won't need the airgapped machine anymore, so power it off
# to make sure there is no remaining trace of the private key.
# The yubikey is ready for signing!
# Note that for verification of the signatures, you need the public key.
# This can be exported from the yubikey as explained below.



#
# Export Public key from a yubikey
#
# Note: only the PUBLIC key can be exported, the private key cannot.
#

# Step 1: export the certificate
yubico-piv-tool -aread-cert -s 9c > yubi.crt

# Step 2: get the public key from the certificate
openssl x509 -in yubi.crt -pubkey -noout > yubi.pub.pem
