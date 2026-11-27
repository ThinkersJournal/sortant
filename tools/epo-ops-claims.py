#!/usr/bin/env python3
"""Compare Google Patents claim text with EPO OPS claim text, claim by claim.
usage: with-secret EPO_OPS_CONSUMER_KEY ... -- with-secret EPO_OPS_CONSUMER_SECRET ... -- \
       python tools/epo-ops-claims.py OUTDIR  GOOGLE_NUM:DOCDB [...]
  e.g. US8838606B1:US.8838606.B1   US2025/0094398A1 is spelled US2025094398A1:US.20250094398.A1
Reads EPO_OPS_CONSUMER_KEY / EPO_OPS_CONSUMER_SECRET from the environment, never prints or writes them.
Writes OUTDIR/<google>.ops.txt and .google.txt (claim text only) and prints a per-claim same/differs table.
Normalises whitespace and leading claim numbering only. Stdlib only. Needs tools/patent-claims.py beside it (or on ..)."""
import sys, os, re, base64, json, subprocess, difflib, urllib.request, urllib.error, urllib.parse
import xml.etree.ElementTree as ET
here = os.path.dirname(os.path.abspath(__file__))
def ops_token():
    k, s = os.environ.get("EPO_OPS_CONSUMER_KEY"), os.environ.get("EPO_OPS_CONSUMER_SECRET")
    if not k or not s: sys.exit("EPO_OPS_CONSUMER_KEY / _SECRET not in the environment")
    auth = base64.b64encode(f"{k}:{s}".encode()).decode()
    req = urllib.request.Request("https://ops.epo.org/3.2/auth/accesstoken", data=b"grant_type=client_credentials",
        headers={"Authorization": "Basic " + auth, "Content-Type": "application/x-www-form-urlencoded"})
    try:
        with urllib.request.urlopen(req, timeout=45) as r:
            return json.load(r)["access_token"]
    except urllib.error.HTTPError as e:
        sys.exit(f"OPS auth refused: HTTP {e.code} {e.read()[:300]!r}")
def ops_claims(tok, docdb):
    url = f"https://ops.epo.org/3.2/rest-services/published-data/publication/docdb/{docdb}/claims"
    req = urllib.request.Request(url, headers={"Authorization": "Bearer " + tok, "Accept": "application/xml"})
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            body = r.read()
    except urllib.error.HTTPError as e:
        return None, f"HTTP {e.code} {e.read()[:300]!r}"
    root = ET.fromstring(body)
    claims = []
    for el in root.iter():
        if el.tag.endswith("}claim"):
            claims.append(" ".join("".join(el.itertext()).split()))
    langs = sorted({el.attrib.get("lang", "?") for el in root.iter() if el.tag.endswith("}claims")})
    return (claims, langs), None
def google_claims(num):
    out = subprocess.run([sys.executable, os.path.join(here, "patent-claims.py") if os.path.exists(os.path.join(here, "patent-claims.py")) else os.path.join(here, "..", "..", "tools", "patent-claims.py"), num],
                         capture_output=True, text=True).stdout
    return [m.group(1) for m in re.finditer(r"^   \[\d+\] (.*)$", out, re.M)]
def norm(c):
    c = re.sub(r"^\s*(?:\[?\d+\]?[.)]?\s*)", "", c)   # leading claim number only
    return " ".join(c.split())
def main():
    outdir, pairs = sys.argv[1], sys.argv[2:]
    os.makedirs(outdir, exist_ok=True)
    tok = ops_token()
    for p in pairs:
        g, d = p.split(":")
        gc = [norm(c) for c in google_claims(g)]
        res, err = ops_claims(tok, d)
        print(f"== {g} ({d}): google {len(gc)} claims")
        open(os.path.join(outdir, g.replace('/', '_') + ".google.txt"), "w", encoding="utf-8").write("\n".join(gc))
        if err: print("   OPS: NO TEXT:", err); continue
        oc, langs = res; oc = [norm(c) for c in oc]
        open(os.path.join(outdir, g.replace('/', '_') + ".ops.txt"), "w", encoding="utf-8").write("\n".join(oc))
        print(f"   OPS {len(oc)} claims, lang {langs}")
        for i in range(max(len(gc), len(oc))):
            a, b = (gc[i] if i < len(gc) else None), (oc[i] if i < len(oc) else None)
            if a == b: print(f"   claim {i+1}: same"); continue
            if a is None or b is None: print(f"   claim {i+1}: only in {'OPS' if a is None else 'Google'}"); continue
            aw, bw = a.split(), b.split(); sm = difflib.SequenceMatcher(None, aw, bw, autojunk=False)
            print(f"   claim {i+1}: DIFFERS (word similarity {sm.ratio():.4f})")
            for op, i1, i2, j1, j2 in sm.get_opcodes():
                if op != "equal": print(f"      {op}: google={' '.join(aw[max(0,i1-3):i2+3])!r}  ops={' '.join(bw[max(0,j1-3):j2+3])!r}")
main()
