"""Package checked-in original assets and native Rust sources; Python stdlib only."""
from pathlib import Path
import hashlib, zipfile
root=Path(__file__).resolve().parents[1]
output=root/'dist';output.mkdir(exist_ok=True)
for name,content,native in [('atomic_age.hmod',True,True),('atomic_age.hnmod',False,True),('atomic_age_content.hmod',True,False)]:
    target=output/name
    with zipfile.ZipFile(target,'w',zipfile.ZIP_DEFLATED) as archive:
        if native:
            archive.write(root/'horizon-native.json','horizon-native.json')
            for path in sorted((root/'mixins').rglob('*')):
                if path.is_file() and path.suffix in ('.rs','.toml'):
                    archive.write(path,path.relative_to(root))
        if content:
            for path in sorted((root/'content').rglob('*')):
                if path.is_file():archive.write(path,path.relative_to(root/'content'))
    print(name,target.stat().st_size,hashlib.sha256(target.read_bytes()).hexdigest())
