#include "DesktopServices.h"

#include <dirent.h>
#include <openssl/evp.h>
#include <openssl/pkcs12.h>
#include <openssl/rand.h>
#include <openssl/x509.h>

#include <cstdio>
#include <cstring>
#include <ctime>
#include <string>
#include <vector>

#include "../sdk/dessen_abi.h"

namespace dessen {
namespace {

// A completed crypto result goes into the app buffer with this guard.
int give(const uint8_t* src, size_t n, uint8_t* out, uint32_t cap) {
  if (n > cap) return DSN_ERR_TOO_BIG;
  memcpy(out, src, n);
  return static_cast<int>(n);
}

struct Evp {
  EVP_PKEY_CTX* c = nullptr;
  ~Evp() { if (c) EVP_PKEY_CTX_free(c); }
};

EVP_PKEY* loadPriv(const uint8_t* der, uint32_t n) {
  const uint8_t* p = der;
  return d2i_AutoPrivateKey(nullptr, &p, n); // accepts PKCS#8 and legacy DER
}

EVP_PKEY* loadPub(const uint8_t* der, uint32_t n) {
  const uint8_t* p = der; // try an X.509 cert first, then a bare SPKI public key
  if (X509* x = d2i_X509(nullptr, &p, n)) {
    EVP_PKEY* k = X509_get_pubkey(x);
    X509_free(x);
    return k;
  }
  p = der;
  return d2i_PUBKEY(nullptr, &p, n);
}

const EVP_MD* mdFor(int algo) {
  switch (algo & 0x7f) {
    case DSN_HASH_SHA1: return EVP_sha1();
    case DSN_HASH_SHA256: return EVP_sha256();
    default: return nullptr;
  }
}

// A fixed device secret for secure_wrap on desktop (firmware uses an eFuse key).
const uint8_t kWrapKey[32] = {0x6b, 0x65, 0x79, 0x66, 0x6f, 0x72, 0x64, 0x65, 0x73, 0x6b, 0x74,
                              0x6f, 0x70, 0x77, 0x72, 0x61, 0x70, 0x30, 0x31, 0x32, 0x33, 0x34,
                              0x35, 0x36, 0x37, 0x38, 0x39, 0x61, 0x62, 0x63, 0x64, 0x65};

}  // namespace

DesktopServices::DesktopServices(std::string fsRoot, std::string appDir, std::string netRoot)
    : fsRoot_(std::move(fsRoot)), appDir_(std::move(appDir)), netRoot_(std::move(netRoot)) {}

// ---- core ----
uint32_t DesktopServices::timeNow() { return static_cast<uint32_t>(::time(nullptr)); }

int DesktopServices::clock(uint8_t* out) {
  const time_t now = ::time(nullptr);
  struct tm t;
  localtime_r(&now, &t);
  const unsigned year = static_cast<unsigned>(t.tm_year + 1900);
  out[0] = year & 0xff;
  out[1] = (year >> 8) & 0xff;
  out[2] = static_cast<uint8_t>(t.tm_mon + 1);
  out[3] = static_cast<uint8_t>(t.tm_mday);
  out[4] = static_cast<uint8_t>(t.tm_hour);
  out[5] = static_cast<uint8_t>(t.tm_min);
  out[6] = static_cast<uint8_t>(t.tm_sec);
  out[7] = static_cast<uint8_t>(t.tm_wday);
  return 0;
}
bool DesktopServices::randomBytes(uint8_t* out, uint32_t n) { return RAND_bytes(out, static_cast<int>(n)) == 1; }
int DesktopServices::deviceId(char* out, uint32_t cap) {
  static const char* kId = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
  const size_t n = strlen(kId);
  if (n + 1 > cap) return DSN_ERR_TOO_BIG;
  memcpy(out, kId, n + 1);
  return static_cast<int>(n);
}

// ---- storage ----
std::string DesktopServices::resolve(const char* path) const { return fsRoot_ + path; }

int DesktopServices::fsRead(const char* path, uint8_t* out, uint32_t cap) {
  FILE* f = fopen(resolve(path).c_str(), "rb");
  if (!f) return DSN_ERR_IO;
  std::vector<uint8_t> buf;
  uint8_t chunk[4096];
  size_t r;
  while ((r = fread(chunk, 1, sizeof chunk, f)) > 0) buf.insert(buf.end(), chunk, chunk + r);
  fclose(f);
  return give(buf.data(), buf.size(), out, cap);
}

int DesktopServices::fsWrite(const char* path, const uint8_t* data, uint32_t n) {
  const std::string full = resolve(path), tmp = full + ".tmp";
  FILE* f = fopen(tmp.c_str(), "wb");
  if (!f) return DSN_ERR_IO;
  const bool ok = fwrite(data, 1, n, f) == n;
  fclose(f);
  if (!ok || rename(tmp.c_str(), full.c_str()) != 0) { remove(tmp.c_str()); return DSN_ERR_IO; }
  return 0;
}

int DesktopServices::fsList(const char* path, char* out, uint32_t cap) {
  DIR* d = opendir(resolve(path).c_str());
  if (!d) return DSN_ERR_IO;
  std::string acc;
  for (dirent* e; (e = readdir(d));) {
    if (!strcmp(e->d_name, ".") || !strcmp(e->d_name, "..")) continue;
    acc += e->d_name;
    if (e->d_type == DT_DIR) acc += '/';
    acc += '\n';
  }
  closedir(d);
  return give(reinterpret_cast<const uint8_t*>(acc.data()), acc.size(), reinterpret_cast<uint8_t*>(out), cap);
}

int DesktopServices::fsDelete(const char* path) { return remove(resolve(path).c_str()) == 0 ? 0 : DSN_ERR_IO; }

int DesktopServices::fsAppDir(char* out, uint32_t cap) {
  if (appDir_.size() + 1 > cap) return DSN_ERR_TOO_BIG;
  memcpy(out, appDir_.c_str(), appDir_.size() + 1);
  return static_cast<int>(appDir_.size());
}

int DesktopServices::keystore(const char* path, const uint8_t* key, uint32_t n, uint32_t) {
  return fsWrite((std::string(path) + ".key").c_str(), key, n);
}

int DesktopServices::recents(char* out, uint32_t cap) {
  return give(reinterpret_cast<const uint8_t*>(recentsFixture.data()), recentsFixture.size(),
              reinterpret_cast<uint8_t*>(out), cap);
}

int DesktopServices::fontLoad(const char* path, int size) {
  lastFontPath = path;
  lastFontSize = size;
  return 3;  // DSN_FONT_CUSTOM — desktop has no real rasterizer; text still renders via the trace target
}

int DesktopServices::bookProgress(const char* path) {
  // Pull the percent for `path` out of the recents fixture; -1 if not listed.
  std::string needle = std::string(path) + "\t";
  const size_t at = recentsFixture.find(needle);
  if (at == std::string::npos) return -1;
  const size_t fields = recentsFixture.rfind('\t', recentsFixture.find('\n', at));
  return fields == std::string::npos ? -1 : atoi(recentsFixture.c_str() + fields + 1);
}

// ---- crypto ----
int DesktopServices::hash(int algo, const uint8_t* data, uint32_t n, uint8_t* out, uint32_t cap) {
  const EVP_MD* md = mdFor(algo);
  if (!md) return DSN_ERR_INVALID;
  uint8_t digest[EVP_MAX_MD_SIZE];
  unsigned dl = 0;
  if (EVP_Digest(data, n, digest, &dl, md, nullptr) != 1) return DSN_ERR_INVALID;
  return give(digest, dl, out, cap);
}

int DesktopServices::aesCbc(int flags, const uint8_t* keyiv, const uint8_t* data, uint32_t n, uint8_t* out,
                            uint32_t cap) {
  const bool enc = flags & DSN_AES_ENCRYPT;
  const bool pad = !(flags & DSN_AES_NOPAD);
  EVP_CIPHER_CTX* ctx = EVP_CIPHER_CTX_new();
  if (!ctx) return DSN_ERR_IO;
  int rc = DSN_ERR_INVALID, outl = 0, finl = 0;
  std::vector<uint8_t> buf(n + 16);
  if (EVP_CipherInit_ex(ctx, EVP_aes_128_cbc(), nullptr, keyiv, keyiv + 16, enc) == 1 &&
      (EVP_CIPHER_CTX_set_padding(ctx, pad ? 1 : 0), 1) &&
      EVP_CipherUpdate(ctx, buf.data(), &outl, data, n) == 1 &&
      EVP_CipherFinal_ex(ctx, buf.data() + outl, &finl) == 1) {
    rc = give(buf.data(), outl + finl, out, cap);
  }
  EVP_CIPHER_CTX_free(ctx);
  return rc;
}

int DesktopServices::rsaEncrypt(const uint8_t* cert, uint32_t cn, const uint8_t* data, uint32_t n, uint8_t* out,
                                uint32_t cap) {
  EVP_PKEY* k = loadPub(cert, cn);
  if (!k) return DSN_ERR_INVALID;
  Evp e;
  e.c = EVP_PKEY_CTX_new(k, nullptr);
  size_t outn = cap;
  int rc = DSN_ERR_INVALID;
  if (e.c && EVP_PKEY_encrypt_init(e.c) == 1 && EVP_PKEY_CTX_set_rsa_padding(e.c, RSA_PKCS1_PADDING) == 1 &&
      EVP_PKEY_encrypt(e.c, out, &outn, data, n) == 1) {
    rc = static_cast<int>(outn);
  }
  EVP_PKEY_free(k);
  return rc == DSN_ERR_INVALID ? rc : (static_cast<uint32_t>(rc) > cap ? DSN_ERR_TOO_BIG : rc);
}

int DesktopServices::rsaDecrypt(int flags, const uint8_t* priv, uint32_t pn, const uint8_t* data, uint32_t n,
                                uint8_t* out, uint32_t cap) {
  EVP_PKEY* k = loadPriv(priv, pn);
  if (!k) return DSN_ERR_INVALID;
  Evp e;
  e.c = EVP_PKEY_CTX_new(k, nullptr);
  const int padding = (flags & DSN_RSA_RAW) ? RSA_NO_PADDING : RSA_PKCS1_PADDING;
  size_t outn = cap;
  int rc = DSN_ERR_INVALID;
  if (e.c && EVP_PKEY_decrypt_init(e.c) == 1 && EVP_PKEY_CTX_set_rsa_padding(e.c, padding) == 1 &&
      EVP_PKEY_decrypt(e.c, out, &outn, data, n) == 1) {
    rc = static_cast<int>(outn);
  }
  EVP_PKEY_free(k);
  return rc;
}

int DesktopServices::rsaSign(int algo, const uint8_t* priv, uint32_t pn, const uint8_t* digest, uint32_t dn,
                             uint8_t* out, uint32_t cap) {
  EVP_PKEY* k = loadPriv(priv, pn);
  if (!k) return DSN_ERR_INVALID;
  Evp e;
  e.c = EVP_PKEY_CTX_new(k, nullptr);
  size_t outn = cap;
  int rc = DSN_ERR_INVALID;
  if (e.c && EVP_PKEY_sign_init(e.c) == 1 && EVP_PKEY_CTX_set_rsa_padding(e.c, RSA_PKCS1_PADDING) == 1) {
    // With a signature MD the DigestInfo prefix is added; without it the raw
    // digest is PKCS#1 v1.5 padded as-is (DSN_SIGN_NODIGESTINFO).
    bool ok = true;
    if (!(algo & DSN_SIGN_NODIGESTINFO)) ok = EVP_PKEY_CTX_set_signature_md(e.c, mdFor(algo)) == 1;
    if (ok && EVP_PKEY_sign(e.c, out, &outn, digest, dn) == 1) rc = static_cast<int>(outn);
  }
  EVP_PKEY_free(k);
  return rc;
}

int DesktopServices::pkcs12Parse(const uint8_t* p12d, uint32_t n, const char* pass, uint8_t* out, uint32_t cap) {
  const uint8_t* p = p12d;
  PKCS12* p12 = d2i_PKCS12(nullptr, &p, n);
  if (!p12) return DSN_ERR_INVALID;
  EVP_PKEY* key = nullptr;
  X509* cert = nullptr;
  int rc = DSN_ERR_INVALID;
  if (PKCS12_parse(p12, pass, &key, &cert, nullptr) == 1 && key && cert) {
    uint8_t* keyDer = nullptr;
    uint8_t* certDer = nullptr;
    const int kl = i2d_PrivateKey(key, &keyDer);   // PKCS#8-compatible DER
    const int cl = i2d_X509(cert, &certDer);
    if (kl > 0 && cl > 0 && static_cast<uint32_t>(2 + kl + cl) <= cap && kl < 0x10000) {
      out[0] = (kl >> 8) & 0xff;
      out[1] = kl & 0xff;
      memcpy(out + 2, keyDer, kl);
      memcpy(out + 2 + kl, certDer, cl);
      rc = 2 + kl + cl;
    } else if (kl > 0 && cl > 0) {
      rc = DSN_ERR_TOO_BIG;
    }
    OPENSSL_free(keyDer);
    OPENSSL_free(certDer);
  }
  EVP_PKEY_free(key);
  X509_free(cert);
  PKCS12_free(p12);
  return rc;
}

int DesktopServices::secureWrap(const uint8_t* data, uint32_t n, uint8_t* out, uint32_t cap) {
  if (2u + 12 + n + 16 > cap) return DSN_ERR_TOO_BIG; // ver(1)+pad(1)+nonce(12)+ct+tag(16)
  out[0] = 1;
  out[1] = 0;
  uint8_t* nonce = out + 2;
  RAND_bytes(nonce, 12);
  EVP_CIPHER_CTX* ctx = EVP_CIPHER_CTX_new();
  int outl = 0, finl = 0, rc = DSN_ERR_IO;
  uint8_t* ct = out + 14;
  if (ctx && EVP_EncryptInit_ex(ctx, EVP_aes_256_gcm(), nullptr, kWrapKey, nonce) == 1 &&
      EVP_EncryptUpdate(ctx, ct, &outl, data, n) == 1 && EVP_EncryptFinal_ex(ctx, ct + outl, &finl) == 1 &&
      EVP_CIPHER_CTX_ctrl(ctx, EVP_CTRL_GCM_GET_TAG, 16, ct + n) == 1) {
    rc = static_cast<int>(2 + 12 + n + 16);
  }
  EVP_CIPHER_CTX_free(ctx);
  return rc;
}

int DesktopServices::secureUnwrap(const uint8_t* blob, uint32_t n, uint8_t* out, uint32_t cap) {
  if (n < 2u + 12 + 16 || blob[0] != 1) return DSN_ERR_INVALID;
  const uint32_t ctn = n - 2 - 12 - 16;
  if (ctn > cap) return DSN_ERR_TOO_BIG;
  const uint8_t* nonce = blob + 2;
  const uint8_t* ct = blob + 14;
  uint8_t tag[16];
  memcpy(tag, ct + ctn, 16);
  EVP_CIPHER_CTX* ctx = EVP_CIPHER_CTX_new();
  int outl = 0, finl = 0, rc = DSN_ERR_INVALID;
  if (ctx && EVP_DecryptInit_ex(ctx, EVP_aes_256_gcm(), nullptr, kWrapKey, nonce) == 1 &&
      EVP_DecryptUpdate(ctx, out, &outl, ct, ctn) == 1 &&
      EVP_CIPHER_CTX_ctrl(ctx, EVP_CTRL_GCM_SET_TAG, 16, tag) == 1 && EVP_DecryptFinal_ex(ctx, out + outl, &finl) == 1) {
    rc = outl + finl;
  }
  EVP_CIPHER_CTX_free(ctx);
  return rc; // authentication failure -> DSN_ERR_INVALID
}

// ---- board ----
int DesktopServices::backlightSet(int brightness, int warm) {
  if (brightness >= 0) lastBrightness = brightness > 100 ? 100 : brightness;
  if (warm >= 0) lastWarm = warm > 100 ? 100 : warm;
  return 0;
}
int DesktopServices::backlightGet() {
  return (lastBrightness < 0 ? 0 : lastBrightness) | (lastWarm < 0 ? 0 : lastWarm) << 8;
}
int DesktopServices::battery() { return (batteryPercent & 0xff) | (batteryPower & 0xff) << 8; }
int DesktopServices::haptic(int ms, int intensity) {
  lastHapticMs = ms > 2000 ? 2000 : ms;
  lastHapticIntensity = intensity;
  return 0;
}
int DesktopServices::accel(uint8_t* out) {
  for (int i = 0; i < 3; i++) {
    out[2 * i] = accelXYZ[i] & 0xff;
    out[2 * i + 1] = (accelXYZ[i] >> 8) & 0xff;
  }
  return 6;
}

// ---- async (synchronous on desktop) ----
int DesktopServices::wifiConnectStart() { return 0; }  // desktop is always online

int DesktopServices::netStart(const char* head, const uint8_t*, uint32_t, uint8_t* resp, uint32_t cap) {
  // Fixture: first line "METHOD url"; map to <netRoot>/<host><path> + .status.
  if (netRoot_.empty()) return DSN_ERR_UNAVAILABLE;
  std::string line(head);
  const size_t nl = line.find('\n');
  if (nl != std::string::npos) line = line.substr(0, nl);
  const size_t sp = line.find(' ');
  if (sp == std::string::npos) return DSN_ERR_INVALID;
  std::string url = line.substr(sp + 1);
  const size_t scheme = url.find("://");
  if (scheme != std::string::npos) url = url.substr(scheme + 3);
  const std::string base = netRoot_ + "/" + url;
  FILE* bf = fopen(base.c_str(), "rb");
  if (!bf) return DSN_ERR_IO;
  std::vector<uint8_t> body;
  uint8_t ch[4096];
  size_t r;
  while ((r = fread(ch, 1, sizeof ch, bf)) > 0) body.insert(body.end(), ch, ch + r);
  fclose(bf);
  uint16_t status = 200;
  if (FILE* sf = fopen((base + ".status").c_str(), "r")) {
    int s;
    if (fscanf(sf, "%d", &s) == 1) status = static_cast<uint16_t>(s);
    fclose(sf);
  }
  // resp layout: u16 status (BE), u16 hdrlen (BE, 0 here), body.
  if (4 + body.size() > cap) return DSN_ERR_TOO_BIG;
  resp[0] = status >> 8;
  resp[1] = status & 0xff;
  resp[2] = 0;
  resp[3] = 0;
  memcpy(resp + 4, body.data(), body.size());
  return static_cast<int>(body.size());
}

int DesktopServices::fetchStart(const char* url, const char* dest) {
  if (netRoot_.empty()) return DSN_ERR_UNAVAILABLE;
  std::string u(url);
  const size_t scheme = u.find("://");
  if (scheme != std::string::npos) u = u.substr(scheme + 3);
  FILE* in = fopen((netRoot_ + "/" + u).c_str(), "rb");
  if (!in) return DSN_ERR_IO;
  FILE* out = fopen(resolve(dest).c_str(), "wb");
  if (!out) { fclose(in); return DSN_ERR_IO; }
  uint8_t ch[4096];
  size_t r, total = 0;
  while ((r = fread(ch, 1, sizeof ch, in)) > 0) { fwrite(ch, 1, r, out); total += r; }
  fclose(in);
  fclose(out);
  return static_cast<int>(total);
}

int DesktopServices::textInputStart(const char*, int, char* out, uint32_t cap) {
  if (!fgets(out, static_cast<int>(cap), stdin)) return -1; // cancelled (EOF)
  size_t n = strlen(out);
  if (n && out[n - 1] == '\n') out[--n] = '\0';
  return static_cast<int>(n);
}

int DesktopServices::keygenStart(uint32_t bits, uint8_t* pub, uint32_t pubCap, uint8_t* priv, uint32_t privCap) {
  EVP_PKEY* k = EVP_RSA_gen(bits);
  if (!k) return DSN_ERR_IO;
  uint8_t* pubDer = nullptr;
  uint8_t* privDer = nullptr;
  const int pl = i2d_PUBKEY(k, &pubDer);
  const int sl = i2d_PrivateKey(k, &privDer);
  EVP_PKEY_free(k);
  int rc = DSN_ERR_IO;
  if (pl > 0 && sl > 0) {
    if (static_cast<uint32_t>(pl) > pubCap || static_cast<uint32_t>(sl) > privCap) {
      rc = DSN_ERR_TOO_BIG;
    } else {
      memcpy(pub, pubDer, pl);
      memcpy(priv, privDer, sl);
      rc = pl << 16 | sl; // pubLen << 16 | privLen
    }
  }
  OPENSSL_free(pubDer);
  OPENSSL_free(privDer);
  return rc;
}

}  // namespace dessen
