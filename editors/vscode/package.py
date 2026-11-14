"""Builds a self-contained .vsix for the current platform, without npm.

    python editors/vscode/package.py

Build the binaries first (cargo build --release --manifest-path bootstrap/Cargo.toml).
The package contains the extension, clpp-boot and clpp-vm for this platform in
extension/bin/<platform>-<arch>/, and the compiler sources in extension/compiler/.
Install it with:  code --install-extension editors/vscode/clpp-dev-<version>-<target>.vsix
"""

import json
import pathlib
import platform
import sys
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent
FILES = ["package.json", "extension.js", "client.js", "language-configuration.json", "README.md"]
FOLDERS = ["syntaxes", "snippets", "images"]


def target():
    system = {"win32": "win32", "darwin": "darwin"}.get(sys.platform, "linux")
    machine = platform.machine().lower()
    arch = "arm64" if machine in ("arm64", "aarch64") else "x64"
    return system, arch


def main():
    manifest = json.loads((HERE / "package.json").read_text(encoding="utf-8"))
    system, arch = target()
    exe = ".exe" if system == "win32" else ""
    binaries = [ROOT / "bootstrap" / "target" / "release" / f"{name}{exe}" for name in ("clpp-boot", "clpp-vm")]
    missing = [str(b) for b in binaries if not b.exists()]
    if missing:
        sys.exit("build the binaries first, missing: " + ", ".join(missing))

    name, version, publisher = manifest["name"], manifest["version"], manifest["publisher"]
    platform_id = f"{system}-{arch}"
    out = HERE / f"{name}-{version}-{platform_id}.vsix"

    vsixmanifest = f"""<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011">
  <Metadata>
    <Identity Language="en-US" Id="{name}" Version="{version}" Publisher="{publisher}" TargetPlatform="{platform_id}"/>
    <DisplayName>{manifest["displayName"]}</DisplayName>
    <Description xml:space="preserve">{manifest["description"]}</Description>
    <Categories>{",".join(manifest.get("categories", []))}</Categories>
    <Icon>extension/{manifest.get("icon", "")}</Icon>
    <Properties>
      <Property Id="Microsoft.VisualStudio.Code.Engine" Value="{manifest["engines"]["vscode"]}"/>
    </Properties>
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code"/>
  </Installation>
  <Dependencies/>
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true"/>
    <Asset Type="Microsoft.VisualStudio.Services.Content.Details" Path="extension/README.md" Addressable="true"/>
    <Asset Type="Microsoft.VisualStudio.Services.Icons.Default" Path="extension/{manifest.get("icon", "")}" Addressable="true"/>
  </Assets>
</PackageManifest>
"""
    content_types = """<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension=".json" ContentType="application/json"/>
  <Default Extension=".js" ContentType="application/javascript"/>
  <Default Extension=".md" ContentType="text/markdown"/>
  <Default Extension=".png" ContentType="image/png"/>
  <Default Extension=".clp" ContentType="text/plain"/>
  <Default Extension=".exe" ContentType="application/octet-stream"/>
  <Default Extension=".vsixmanifest" ContentType="text/xml"/>
  <Default Extension=".xml" ContentType="text/xml"/>
  <Override PartName="/extension/bin/""" + platform_id + """/clpp-boot" ContentType="application/octet-stream"/>
  <Override PartName="/extension/bin/""" + platform_id + """/clpp-vm" ContentType="application/octet-stream"/>
</Types>
"""
    count = 0
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("extension.vsixmanifest", vsixmanifest)
        z.writestr("[Content_Types].xml", content_types)
        for f in FILES:
            z.write(HERE / f, f"extension/{f}")
            count += 1
        z.write(ROOT / "LICENSE", "extension/LICENSE")
        count += 1
        for folder in FOLDERS:
            for p in sorted((HERE / folder).rglob("*")):
                if p.is_file():
                    z.write(p, f"extension/{p.relative_to(HERE).as_posix()}")
                    count += 1
        for b in binaries:
            info = zipfile.ZipInfo.from_file(b, f"extension/bin/{platform_id}/{b.name}")
            info.external_attr = 0o755 << 16  # keep the executable bit on Linux and macOS
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, b.read_bytes())
            count += 1
        for p in sorted((ROOT / "src").rglob("*.clp")):
            z.write(p, f"extension/compiler/{p.relative_to(ROOT / 'src').as_posix()}")
            count += 1
    size = out.stat().st_size / 1024 / 1024
    print(f"wrote {out.relative_to(ROOT)} ({count} files, {size:.1f} MB)")
    print(f"install with: code --install-extension {out.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
