from pathlib import Path
import json,shutil,zipfile,hashlib,sys
root=Path(__file__).resolve().parents[1]
exe=Path(sys.argv[1]);assert exe.read_bytes()[:2]==b'MZ'
output=root/'dist';output.mkdir(exist_ok=True)
package=output/'Atomic-Age-0.5.0-Windows.zip'
with zipfile.ZipFile(package,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as z:
    z.write(exe,'horizon-client.exe')
    z.write(output/'atomic_age_content.hmod','atomic_age_content.hmod')
    z.write(root/'README_RU.md','README_RU.md')
    z.write(root/'LICENSE','LICENSE')
    for p in (root/'packaging').iterdir():z.write(p,p.name)
print(package,hashlib.sha256(package.read_bytes()).hexdigest())

source=output/'Atomic-Age-0.5.0-Source.zip'
with zipfile.ZipFile(source,'w',zipfile.ZIP_DEFLATED) as z:
    for p in sorted(root.rglob('*')):
        if p.is_file() and p.relative_to(root).parts[0]!='dist' and p.name!='plan.json' and '__pycache__' not in p.parts:
            z.write(p,Path('atomic-age-native')/p.relative_to(root))
