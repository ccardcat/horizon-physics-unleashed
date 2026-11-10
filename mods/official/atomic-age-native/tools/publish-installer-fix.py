"""Repackage the verified existing client with the tested installer; never rebuild or replace its code."""
import hashlib, io, json, os, pathlib, subprocess, tempfile, time, zipfile
ROOT=pathlib.Path(__file__).resolve().parents[1]
REPO=os.environ.get('GITHUB_REPOSITORY','ccardcat/horizon-physics-unleashed')
TAG='atomic-age-v0.5.0'
CLIENT_SHA='6be5f1ad81b8feb70528427f1ea6ddd83a69ae9cd1968bf59d2a1ea928afa56e'

def run(args,**kwargs):
    return subprocess.run(args,check=True,**kwargs)
def api(path,method='GET',payload=None):
    args=['gh','api',f'repos/{REPO}/{path}','--method',method]
    if payload is not None:args+=['--input','-']
    r=run(args,input=json.dumps(payload) if payload is not None else None,capture_output=True,text=True)
    return json.loads(r.stdout)
def replace_entries(path,replacements):
    with zipfile.ZipFile(path) as source:
        assert source.testzip() is None
        old=[(info,source.read(info.filename)) for info in source.infolist()]
    pending=dict(replacements)
    with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as target:
        for info,data in old:
            if info.filename in pending:
                data=pending.pop(info.filename);info.date_time=time.localtime()[:6]
            target.writestr(info,data)
        for name,data in pending.items():target.writestr(name,data)

release=api(f'releases/tags/{TAG}')
assert not release['draft']
with tempfile.TemporaryDirectory(prefix='atomic-installer-release-') as folder:
    out=pathlib.Path(folder)
    run(['gh','release','download',TAG,'--repo',REPO,'--dir',str(out)])
    for line in (out/'SHA256SUMS.txt').read_text(encoding='utf-8').splitlines():
        digest,name=line.split(None,1);name=pathlib.PurePosixPath(name.strip()).name
        assert hashlib.sha256((out/name).read_bytes()).hexdigest()==digest,name
    windows=out/'Atomic-Age-0.5.0-Windows.zip'
    with zipfile.ZipFile(windows) as z:
        assert hashlib.sha256(z.read('horizon-client.exe')).hexdigest()==CLIENT_SHA
    installer=(ROOT/'packaging/install.ps1').read_bytes()
    assert all(b<128 for b in installer)
    readme=(ROOT/'README_RU.md').read_bytes()
    replace_entries(windows,{'install.ps1':installer,'README_RU.md':readme})
    source=out/'Atomic-Age-0.5.0-Source.zip'
    replace_entries(source,{
        'atomic-age-native/packaging/install.ps1':installer,
        'atomic-age-native/README_RU.md':readme,
        'atomic-age-native/tests/installer-windows.ps1':(ROOT/'tests/installer-windows.ps1').read_bytes(),
    })
    patch=out/'Atomic-Age-0.5.0-Installer-Fix.zip'
    with zipfile.ZipFile(patch,'w',zipfile.ZIP_DEFLATED) as z:
        z.writestr('install.ps1',installer)
        z.write(ROOT/'packaging/Install.cmd','Install.cmd')
    (out/'install.ps1').write_bytes(installer)
    with zipfile.ZipFile(windows) as z:
        assert z.testzip() is None
        assert z.read('install.ps1')==installer
        assert hashlib.sha256(z.read('horizon-client.exe')).hexdigest()==CLIENT_SHA
    sums=out/'SHA256SUMS.txt'
    sums.write_text(''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n' for p in sorted(out.iterdir()) if p.is_file() and p.name!=sums.name),encoding='utf-8')
    files=[patch,out/'install.ps1',windows,source,sums]
    run(['gh','release','upload',TAG,'--repo',REPO,'--clobber',*[str(p) for p in files]])
    marker='<!-- atomic-installer-fix -->'
    body=release.get('body') or ''
    if marker in body:body=body.split('<!-- /atomic-installer-fix -->',1)[-1].lstrip()
    prefix=(marker+'\n**Исправление установщика:** [маленький ZIP для уже распакованной сборки]'
        f'(https://github.com/{REPO}/releases/download/{TAG}/{patch.name}). Распакуй поверх папки Atomic Age и замени `install.ps1`, затем запусти `Install.cmd`. Полный Windows ZIP также обновлён; игровой EXE сохранён без изменений.\n\n'
        'Ошибка `os error 112` означает нехватку места в хранилище Horizon. Перенеси данные штатной кнопкой **Настройки → Папка данных → Перенести…** в новую пустую папку на диске с достаточным местом, затем повтори установку.\n<!-- /atomic-installer-fix -->\n\n')
    api(f'releases/{release["id"]}','PATCH',{'tag_name':TAG,'body':prefix+body})
    print('Updated installer assets; preserved Windows client SHA256',CLIENT_SHA)
