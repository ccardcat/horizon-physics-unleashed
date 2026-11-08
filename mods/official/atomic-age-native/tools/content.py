#!/usr/bin/env python3
"""Original Atomic Age item assets and combined native/content package."""
from pathlib import Path
import json, shutil, zipfile, hashlib
ROOT=Path(__file__).resolve().parents[1]
content=ROOT/'content'
if not content.exists():
    raise SystemExit('The checked-in content/ folder is required.')
(content/'README-RU.txt').unlink(missing_ok=True)
for folder in [content/'data/atomic_age/function',content/'data/minecraft']:
    if folder.exists(): shutil.rmtree(folder)
(content/'horizon-mod.toml').write_text('''id = "atomic_age"
name = "Atomic Age Native"
version = "0.5.0"
description = "Native nuclear shockwaves and mushroom clouds, destructive red laser, live two-camera portals and six bombs. Requires a launcher native build."
authors = ["ccardcat"]
api = "0.2"
side = "both"
capabilities = ["registries"]
''')
def write(path,obj):
    path=content/path;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(json.dumps(obj,ensure_ascii=False,indent=2)+'\n')
tools=['detonator','laser_cannon','portal_gun','portal_reset','emergency_stop']
for item in tools:
    write(Path(f'data/atomic_age/horizon/item/{item}.json'),dict(template='minecraft:stick',max_stack_size=1,rarity='epic',components={'minecraft:lore':[{'translate':f'atomic_age.{item}.hint'}]}))
    write(Path(f'assets/atomic_age/items/{item}.json'),{'model':{'type':'minecraft:model','model':f'atomic_age:item/{item}'}})
# Cuboid gun models, drawn with original solid-color textures. No Portal assets are copied.
from PIL import Image, ImageDraw
palette={'shell':(228,228,220),'dark':(29,32,39),'red':(255,24,12),'blue':(25,125,255),'orange':(255,115,15),'metal':(105,116,128)}
for name,color in palette.items():
    image=Image.new('RGBA',(16,16),color+(255,));d=ImageDraw.Draw(image)
    for y in range(16):
        for x in range(16):
            if (x+y)%7==0:d.point((x,y),fill=tuple(max(0,c-14) for c in color)+(255,))
    p=content/f'assets/atomic_age/textures/item/{name}.png';p.parent.mkdir(parents=True,exist_ok=True);image.save(p)
def box(a,b,texture,light=0):
    return {'from':a,'to':b,'light_emission':light,'faces':{face:{'texture':'#'+texture}for face in ['north','south','east','west','up','down']}}
def gun(portal):
    glow='blue' if portal else 'red'
    return {'textures':{k:f'atomic_age:item/{k}'for k in palette},'elements':[
        box([3,5,2],[13,12,11],'shell' if portal else 'metal'),box([4,6,0],[12,11,3],'dark'),
        box([5,7,-1],[11,10,0],glow,15),box([6,0,8],[10,6,12],'dark'),
        box([6,8,3],[10,11,10],glow,15),
        box([2,4,-2],[4,6,4],'dark'),box([12,4,-2],[14,6,4],'dark'),box([7,12,-2],[9,14,4],'dark')
        ] if portal else [box([3,5,2],[13,12,11],'metal'),box([4,6,-3],[12,11,4],'dark'),box([5,7,-4],[11,10,-3],glow,15),box([6,0,8],[10,6,12],'dark'),box([6,12,4],[10,14,10],glow,15)],
        'display':{'firstperson_righthand':{'rotation':[0,-90,18],'translation':[2,1,-3],'scale':[0.65,0.65,0.65]},'thirdperson_righthand':{'rotation':[0,-90,0],'translation':[0,3,1],'scale':[0.5,0.5,0.5]},'gui':{'rotation':[25,-35,0],'translation':[0,0,0],'scale':[0.8,0.8,0.8]}}}
for item,portal in [('laser_cannon',False),('portal_gun',True)]:write(Path(f'assets/atomic_age/models/item/{item}.json'),gun(portal))
for item,color in [('portal_reset','blue'),('emergency_stop','red')]:
    write(Path(f'assets/atomic_age/models/item/{item}.json'),{'textures':{'all':f'atomic_age:item/{color}'},'elements':[box([3,1,4],[13,10,12],'all',5),box([5,10,6],[11,14,10],'all',15)]})
write(Path('data/atomic_age/horizon/creative_tab/tools.json'),{'title':'itemGroup.atomic_age.tools','icon':'atomic_age:portal_gun','items':['atomic_age:'+i for i in tools]})
ru={
 'itemGroup.atomic_age.tools':'Atomic Age — оружие и порталы',
 'item.atomic_age.laser_cannon':'Лазерная пушка', 'item.atomic_age.portal_gun':'Портальная пушка',
 'item.atomic_age.portal_reset':'Сброс порталов','item.atomic_age.emergency_stop':'Аварийный стоп',
 'atomic_age.detonator.hint':'ПКМ по бомбе в прицеле: взрыв через 3 секунды',
 'atomic_age.laser_cannon.hint':'Удерживай ПКМ: красный луч прожигает блоки (96 м)',
 'atomic_age.portal_gun.hint':'ПКМ: синий портал; Shift + ПКМ: оранжевый. Стена высотой 2 блока.',
 'atomic_age.portal_reset.hint':'ПКМ: убрать оба своих портала',
 'atomic_age.emergency_stop.hint':'ПКМ: остановить все активные бомбы в этом измерении',
 'atomic_age.bomb.hint':'Поставь и активируй детонатором. Разрушает мир.',
 'atomic_age.busy':'Перезарядка или слишком много активных взрывов',
 'atomic_age.armed':'ЗАРЯД АКТИВИРОВАН — 3 СЕКУНДЫ',
 'atomic_age.wall_only':'Портал ставится на вертикальную стену',
 'atomic_age.portal_surface':'Нужны два сплошных блока без контейнеров',
}
en={
 'itemGroup.atomic_age.tools':'Atomic Age — weapons & portals',
 'item.atomic_age.laser_cannon':'Laser Cannon','item.atomic_age.portal_gun':'Portal Gun',
 'item.atomic_age.portal_reset':'Portal Reset','item.atomic_age.emergency_stop':'Emergency Stop',
 'atomic_age.detonator.hint':'Right-click a bomb in your sights: 3-second fuse',
 'atomic_age.laser_cannon.hint':'Hold right-click: red beam burns blocks (96 blocks)',
 'atomic_age.portal_gun.hint':'Right-click: blue; Shift + right-click: orange. Use a two-block-tall wall.',
 'atomic_age.portal_reset.hint':'Right-click: remove your portal pair',
 'atomic_age.emergency_stop.hint':'Right-click: stop all active bombs in this dimension',
 'atomic_age.bomb.hint':'Place and activate with the detonator. Destroys terrain.',
 'atomic_age.busy':'Recharging or too many active blasts',
 'atomic_age.armed':'CHARGE ARMED — 3 SECONDS',
 'atomic_age.wall_only':'Place a portal on a vertical wall',
 'atomic_age.portal_surface':'Needs two solid blocks without a container',
}
for locale,addition in [('ru_ru',ru),('en_us',en)]:
    p=Path(f'assets/atomic_age/lang/{locale}.json');current=json.loads((content/p).read_text());current.update(addition);write(p,current)
if __name__=='__main__':
    output=ROOT/'dist';output.mkdir(exist_ok=True)
    for filename,hybrid in [('atomic_age.hmod',True),('atomic_age.hnmod',False)]:
        target=output/filename
        with zipfile.ZipFile(target,'w',zipfile.ZIP_DEFLATED) as z:
            z.write(ROOT/'horizon-native.json','horizon-native.json')
            for p in sorted((ROOT/'mixins').rglob('*')):
                if p.is_file() and p.suffix in ('.rs','.toml'):z.write(p,p.relative_to(ROOT))
            if hybrid:
                for p in sorted(content.rglob('*')):
                    if p.is_file():z.write(p,p.relative_to(content))
        print(filename,target.stat().st_size,hashlib.sha256(target.read_bytes()).hexdigest())
    with zipfile.ZipFile(output/'atomic_age_content.hmod','w',zipfile.ZIP_DEFLATED) as z:
        for p in sorted(content.rglob('*')):
            if p.is_file():z.write(p,p.relative_to(content))
