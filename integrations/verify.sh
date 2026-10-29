#!/usr/bin/env bash
# Verifies the integrations. Static checks always run; live MCP checks run only if the binary is found.
# Usage: integrations/verify.sh [path/to/agentic-ooxml-mcp]
# Exit 1 on any failure. SKIP lines are not failures but mean "not tested".
set -u
cd "$(dirname "$0")/.."
fail=0; ok(){ echo "PASS  $*"; }; bad(){ echo "FAIL  $*"; fail=1; }; skip(){ echo "SKIP  $*"; }
MCP="${1:-}"
[ -z "$MCP" ] && for c in target/debug/agentic-ooxml-mcp target/release/agentic-ooxml-mcp; do [ -x "$c" ] && MCP="$PWD/$c" && break
done
[ -z "$MCP" ] && command -v agentic-ooxml-mcp >/dev/null 2>&1 && MCP="$(command -v agentic-ooxml-mcp)"

# 1. JSON files parse
for f in .claude-plugin/marketplace.json integrations/claude-code/.claude-plugin/plugin.json integrations/claude-code/.mcp.json integrations/mcp/mcp.json integrations/cursor/mcp.json; do
  python3 -c 'import json,sys;json.load(open(sys.argv[1]))' "$f" 2>/dev/null && ok "json $f" || bad "json $f"
done

# 2. Skills: spec constraints and plugin copy in sync
python3 -I - <<'PY' || fail=1
import re,os,sys
bad=0
root="integrations/skills"
for d in sorted(os.listdir(root)):
    p=os.path.join(root,d,"SKILL.md")
    if not os.path.isfile(p): continue
    t=open(p).read()
    m=re.match(r"---\n(.*?)\n---\n",t,re.S)
    fm=dict(re.findall(r"^(\w+):\s*(.*)$",m.group(1),re.M)) if m else {}
    errs=[]
    if fm.get("name")!=d: errs.append("name!=dir")
    if not re.fullmatch(r"[a-z0-9]+(-[a-z0-9]+)*",fm.get("name","")) or len(fm.get("name",""))>64: errs.append("bad name")
    if not 1<=len(fm.get("description",""))<=1024: errs.append("description length")
    if t.count("\n")>500: errs.append(">500 lines")
    print(("FAIL  " if errs else "PASS  ")+"skill "+d+" "+",".join(errs)); bad|=bool(errs)
sys.exit(bad)
PY
diff -r integrations/skills integrations/claude-code/skills >/dev/null && ok "plugin skills identical to integrations/skills" || bad "plugin skills differ: rerun: rm -rf integrations/claude-code/skills && cp -R integrations/skills integrations/claude-code/skills"
# Skills may only name contract tools
bad_tools=$(grep -rhoE 'ao_[a-z_]+' integrations/skills | sort -u | grep -vxE 'ao_(inspect|list|read|sources|validate|diff|write|reconcile)' || true)
[ -z "$bad_tools" ] && ok "skills reference only contract tools" || bad "unknown tools in skills: $bad_tools"

# 3. Claude Code validator
if command -v claude >/dev/null 2>&1; then
  claude plugin validate integrations/claude-code >/dev/null 2>&1 && ok "claude plugin validate (plugin)" || bad "claude plugin validate (plugin)"
  claude plugin validate . >/dev/null 2>&1 && ok "claude plugin validate (marketplace)" || bad "claude plugin validate (marketplace)"
else skip "claude CLI not found"; fi
command -v skills-ref >/dev/null 2>&1 && { for d in integrations/skills/ooxml-*; do skills-ref validate "$d" >/dev/null 2>&1 && ok "skills-ref $d" || bad "skills-ref $d"; done; } || skip "skills-ref not installed (npx/pip package from agentskills/agentskills)"

# 4. Live MCP
if [ -z "$MCP" ]; then skip "agentic-ooxml-mcp binary not found: live MCP checks NOT run"; else
  echo "using $MCP"
  tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
  python3 -I - "$MCP" "$tmp" <<'PY' || fail=1
import json,subprocess,sys,os,zipfile
mcp,tmp=sys.argv[1],sys.argv[2]
doc=os.path.join(tmp,"t.docx")
with zipfile.ZipFile(doc,"w") as z:
    z.writestr("[Content_Types].xml",'<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>')
    z.writestr("_rels/.rels",'<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>')
    z.writestr("word/document.xml",'<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Q1 17.2</w:t></w:r></w:p></w:body></w:document>')
def run(env_extra, calls):
    env=dict(os.environ,AGENTIC_OOXML_ROOTS=tmp,**env_extra)
    p=subprocess.Popen([mcp],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=env,cwd=tmp)
    out=[]
    for i,(m,params) in enumerate(calls,1):
        msg={"jsonrpc":"2.0","method":m,"params":params}
        if not m.startswith("notifications/"): msg["id"]=i
        p.stdin.write(json.dumps(msg)+"\n"); p.stdin.flush()
        if "id" in msg: out.append(json.loads(p.stdout.readline()))
    p.stdin.close(); p.wait(timeout=10); return out
init=("initialize",{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"verify","version":"0"}})
note=("notifications/initialized",{})
fail=0
def chk(name,cond):
    global fail; print(("PASS  " if cond else "FAIL  ")+name); fail|=not cond
r=run({},[init,note,("tools/list",{}),("tools/call",{"name":"ao_inspect","arguments":{"file":doc}}),
          ("tools/call",{"name":"ao_validate","arguments":{"file":doc}}),
          ("tools/call",{"name":"ao_write","arguments":{"file":doc,"kind":"intent","input":{"sections":[{"id":"a","text":"x"}]}}})])
chk("initialize handshake",r[0].get("result",{}).get("serverInfo") is not None)
names={t["name"] for t in r[1].get("result",{}).get("tools",[])}
want={"ao_inspect","ao_list","ao_read","ao_sources","ao_validate","ao_diff","ao_write","ao_reconcile"}
chk("tools/list has all 8 contract tools (missing: %s)"%sorted(want-names),want<=names)
chk("tools/call ao_inspect succeeds",not r[2].get("error") and not r[2].get("result",{}).get("isError"))
chk("tools/call ao_validate responds",not r[3].get("error"))
w=json.dumps(r[4])
chk("ao_write refused without AGENTIC_OOXML_ALLOW_WRITE",bool(r[4].get("error")) or r[4].get("result",{}).get("isError"))
r2=run({},[init,note,("tools/call",{"name":"ao_inspect","arguments":{"file":"/etc/hosts"}})])
chk("path outside AGENTIC_OOXML_ROOTS refused",bool(r2[1].get("error")) or r2[1].get("result",{}).get("isError"))
sys.exit(fail)
PY
fi
exit $fail
