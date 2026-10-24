"""Package resolution, install planning and repo validation for SigmaOS-packages.

Principles: fail-fast on missing dependencies and cycles, deterministic
alphabetical tie-breaking, pure plan layer (nothing is downloaded or
installed by these functions), KISS dependency model.
"""
import argparse, json, re, sys

ENGINE = {
    "name": "sigma-pkg",
    "summary": "Resolve dependencies, plan installs, verify systems",
    "language": "python",
    "verbs": ["resolve", "install-plan", "verify", "repo"],
    "principles": ["Fail-Fast", "Pure Plan / Thin Execute", "KISS",
                   "DRY", "Deterministic Testing"],
}

cli_doc = """\
$ sigma-pkg resolve repo.json base desktop
$ sigma-pkg install-plan repo.json base
$ sigma-pkg verify installed.json repo.json base
$ sigma-pkg repo sigma-main https://repo.sigmaos.example/stable --trusted
"""


class PkgError(Exception):
    pass


_NAME_RE = re.compile(r"^[a-z0-9][a-z0-9+.-]*$")


def validate_repo(repo):
    """Pure: structural validation of a repo dict {name: {deps, version}}."""
    if not isinstance(repo, dict):
        raise PkgError("repo must be an object")
    for name, spec in repo.items():
        if not _NAME_RE.match(name):
            raise PkgError(f"bad package name: {name!r}")
        if not isinstance(spec, dict) or not spec.get("version"):
            raise PkgError(f"{name}: missing version")
        deps = spec.get("deps", [])
        if not isinstance(deps, list) or not all(isinstance(d, str) for d in deps):
            raise PkgError(f"{name}: deps must be a list of names")
        if len(set(deps)) != len(deps):
            raise PkgError(f"{name}: duplicate dependency")
    for name, spec in repo.items():
        for dep in spec.get("deps", []):
            if dep not in repo:
                raise PkgError(f"{name}: dependency not in repo: {dep}")
    return repo


def resolve(repo, roots):
    """Pure: dependency-ordered install list (deps first, alphabetical ties)."""
    validate_repo(repo)
    order, done, stack = [], set(), []

    def visit(name, chain):
        if name in done:
            return
        if name in stack:
            raise PkgError("dependency cycle: " + " -> ".join(chain + [name]))
        if name not in repo:
            raise PkgError(f"unknown package: {name}")
        stack.append(name)
        for dep in sorted(repo[name].get("deps", [])):
            visit(dep, chain + [name])
        stack.pop()
        done.add(name)
        order.append(name)

    if not roots:
        raise PkgError("no root packages given")
    for root in sorted(roots):
        visit(root, [])
    return order


def install_plan(repo, roots):
    """Pure: ordered install steps plus a final verification step."""
    order = resolve(repo, roots)
    steps = [{"action": "install", "pkg": p, "version": repo[p]["version"]}
             for p in order]
    steps.append({"action": "verify", "pkg": None, "version": None})
    return steps


def verify(installed, repo, roots):
    """Pure: check that an installed {pkg: version} map satisfies the roots."""
    resolved = resolve(repo, roots)
    missing = [p for p in resolved if p not in installed]
    outdated = [{"pkg": p, "installed": installed[p], "repo": repo[p]["version"]}
                for p in resolved
                if p in installed and installed[p] != repo[p]["version"]]
    return {"ok": not missing and not outdated, "missing": missing,
            "outdated": outdated}


def repo_entry(name, url, trusted=False):
    """Pure: validate a repository registration entry."""
    if not _NAME_RE.match(name):
        raise PkgError(f"bad repo name: {name!r}")
    if not isinstance(url, str) or not (url.startswith("https://")
                                        or url.startswith("file://")):
        raise PkgError(f"repo url must be https:// or file://: {url!r}")
    return {"name": name, "url": url, "trusted": bool(trusted)}


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog=ENGINE["name"], description=ENGINE["summary"])
    sub = p.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("resolve", help="dependency-ordered package list")
    r.add_argument("repo", help="JSON repo object")
    r.add_argument("roots", nargs="+")
    i = sub.add_parser("install-plan", help="ordered install steps")
    i.add_argument("repo", help="JSON repo object")
    i.add_argument("roots", nargs="+")
    v = sub.add_parser("verify", help="verify an installed package map")
    v.add_argument("installed", help="JSON object: pkg -> version")
    v.add_argument("repo", help="JSON repo object")
    v.add_argument("roots", nargs="+")
    rp = sub.add_parser("repo", help="validate a repo entry")
    rp.add_argument("name")
    rp.add_argument("url")
    rp.add_argument("--trusted", action="store_true")
    return p


def _load(path):
    with open(path, "r", encoding="utf-8") as fh:
        return json.load(fh)


def main(argv=None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.cmd == "repo":
            entry = repo_entry(args.name, args.url, args.trusted)
            print(f"{entry['name']}: {entry['url']} "
                  f"({'trusted' if entry['trusted'] else 'untrusted'})")
            return 0
        repo = validate_repo(_load(args.repo))
        if args.cmd == "resolve":
            for pkg in resolve(repo, args.roots):
                print(pkg)
        elif args.cmd == "install-plan":
            for step in install_plan(repo, args.roots):
                if step["action"] == "install":
                    print(f"install {step['pkg']} {step['version']}")
                else:
                    print("verify")
        elif args.cmd == "verify":
            result = verify(_load(args.installed), repo, args.roots)
            for pkg in result["missing"]:
                print(f"missing {pkg}")
            for o in result["outdated"]:
                print(f"outdated {o['pkg']}: {o['installed']} -> {o['repo']}")
            print("ok" if result["ok"] else "not ok")
            return 0 if result["ok"] else 1
        return 0
    except (PkgError, OSError, ValueError, json.JSONDecodeError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 1


def self_test() -> None:
    # diamond: base -> (libc, zlib), libc -> kernel-headers, zlib -> kernel-headers
    repo = {
        "base": {"deps": ["libc", "zlib"], "version": "1.0"},
        "libc": {"deps": ["kernel-headers"], "version": "2.3"},
        "zlib": {"deps": ["kernel-headers"], "version": "1.3"},
        "kernel-headers": {"deps": [], "version": "6.9"},
        "desktop": {"deps": ["base", "wayland"], "version": "0.9"},
        "wayland": {"deps": ["libc"], "version": "1.22"},
    }
    validate_repo(repo)
    order = resolve(repo, ["base"])
    assert order == ["kernel-headers", "libc", "zlib", "base"], order
    assert order.index("kernel-headers") < order.index("zlib") < order.index("base")
    big = resolve(repo, ["desktop"])
    assert big[-1] == "desktop" and big[0] == "kernel-headers"
    # install plan: installs before final verify
    plan = install_plan(repo, ["base"])
    assert [s["pkg"] for s in plan[:-1]] == order
    assert plan[-1]["action"] == "verify"
    # verify: satisfied vs missing vs outdated
    full = {p: repo[p]["version"] for p in order}
    assert verify(full, repo, ["base"])["ok"] is True
    partial = dict(full)
    del partial["zlib"]
    res = verify(partial, repo, ["base"])
    assert res["ok"] is False and res["missing"] == ["zlib"]
    stale = dict(full)
    stale["libc"] = "2.2"
    res = verify(stale, repo, ["base"])
    assert res["ok"] is False and res["outdated"] == [
        {"pkg": "libc", "installed": "2.2", "repo": "2.3"}]
    # repo entries
    entry = repo_entry("sigma-main", "https://repo.example/stable", trusted=True)
    assert entry["trusted"] is True
    for bad_call in (
        lambda: repo_entry("Bad Name", "https://x"),
        lambda: repo_entry("ok", "http://insecure.example"),
        lambda: repo_entry("ok", "ftp://old.example"),
        lambda: resolve(repo, []),
        lambda: resolve(repo, ["ghost"]),
        lambda: resolve({"a": {"deps": ["b"], "version": "1"},
                         "b": {"deps": ["a"], "version": "1"}}, ["a"]),
        lambda: validate_repo({"A Bad": {"version": "1"}}),
        lambda: validate_repo({"x": {"version": ""}}),
        lambda: validate_repo({"x": {"version": "1", "deps": ["ghost"]}}),
        lambda: validate_repo({"x": {"version": "1", "deps": ["x", "x"]}}),
    ):
        try:
            bad_call()
        except PkgError:
            pass
        else:
            raise AssertionError("expected PkgError")
    # CLI smoke through main()
    import os, tempfile
    with tempfile.TemporaryDirectory() as d:
        rpath = os.path.join(d, "repo.json")
        with open(rpath, "w", encoding="utf-8") as fh:
            json.dump(repo, fh)
        ipath = os.path.join(d, "installed.json")
        with open(ipath, "w", encoding="utf-8") as fh:
            json.dump(full, fh)
        assert main(["resolve", rpath, "base"]) == 0
        assert main(["install-plan", rpath, "desktop"]) == 0
        assert main(["verify", ipath, rpath, "base"]) == 0
        assert main(["repo", "sigma-main", "https://repo.example/stable"]) == 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
