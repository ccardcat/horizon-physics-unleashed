#!/usr/bin/env python3
"""Assemble original native modules/helpers into a launcher mixin bundle."""
from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[1]
B=ROOT/'mixins/atomic_age'
(B/'renderer.rs').write_text((B/'portal_math.rs').read_text().replace('//! Original Atomic Age portal geometry. Coordinates are in blocks, Y is up.','// Original Atomic Age portal geometry. Coordinates are in blocks, Y is up.')+'\n'+(B/'render.rs').read_text()+'\n'+(B/'subframe.rs').read_text())
manifest='''[mod]
id = "atomic_age"
version = "0.5.0"

[compatibility]
horizon_revision = "fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca"

[[module]]
id = "gameplay"
file = "crates/horizon-client/src/game.rs"
name = "atomic_age"
source = "game.rs"

[[module]]
id = "renderer"
file = "crates/horizon-client-render/src/gfx.rs"
name = "atomic_age"
source = "renderer.rs"
visibility = "pub"
'''
def hook(id,target,params,body,kind='HEAD',ret=''):
    global manifest
    args=[]; bindings=[]
    for name,ty,value in params:
        args.append(f'{name}: {ty}')
        bindings.append('{name = '+json.dumps(name)+', ty = '+json.dumps(ty)+', value = '+json.dumps(value)+'}')
    (B/f'{id}.rs').write_text('fn run('+', '.join(args)+')'+(' -> '+ret if ret else '')+' {\n    '+body+'\n}\n')
    manifest+='\n[[injection]]\nid = '+json.dumps(id)+'\ntarget = '+json.dumps(target)+'\nkind = '+json.dumps(kind)+'\nhelper = '+json.dumps(id+'.rs')+'\nbindings = ['+', '.join(bindings)+']\n'
G="horizon_client::game::native_hooks::Game<'_>::"
g=('game',"&mut crate::game::Game<'_>",'&mut *self')
s=('stack','&horizon_client::items::ItemStack','stack')
c=('claimed','&mut bool','claimed')
for id,fn in [('use_air','native_use_item'),('use_block','native_use_item_on')]:
    hook(id,G+fn,[g,s,c],'crate::game::atomic_age::use_item(game, stack, claimed);')
hook('duration',G+'native_use_duration',[s,('duration','&mut i32','duration')],'crate::game::atomic_age::duration(stack, duration);')
hook('laser',G+'native_on_use_tick',[g,s],'crate::game::atomic_age::laser(game, stack);')
hook('tick',G+'native_level_tick_post',[g,('dimension','&str','dimension')],'crate::game::atomic_age::level_tick(game, dimension);')
hook('receive',G+'native_net_from_server',[g,('channel','&str','channel'),('data','&[u8]','data')],'crate::game::atomic_age::receive(game, channel, data);')
hook('reset',G+'native_world_lifecycle',[], 'crate::game::atomic_age::reset();')
hook('shake',G+'native_camera_shake',[('game',"&crate::game::Game<'_>",'&*self'),('shake','&mut crate::game::NativeShake','shake')],'crate::game::atomic_age::shake(game, shake);')
hook('prepare','horizon_client_render::gfx::Renderer::render',[('renderer','&mut crate::gfx::Renderer','&mut *self'),('frame',"&crate::gfx::Frame<'_>",'frame')],'crate::gfx::atomic_age::prepare(renderer, frame);')
hook('clip','horizon_client_render::gfx::view::View::screen',[(n,'f32',n) for n in ['yaw','pitch','fov_degrees','aspect','far']], 'crate::gfx::atomic_age::clip_view(crate::gfx::view::View { view:crate::camera::view_matrix(yaw,pitch), projection:crate::gfx::world_projection(fov_degrees,aspect,far), aspect, tan_half_fov:(fov_degrees.to_radians()/2.).tan() })','OVERWRITE','crate::gfx::view::View')
hook('draw','horizon_client_render::gfx::native_render::native_world_drawn',[('world',"&mut crate::gfx::native_render::WorldDrawn<'_>",'world')],'crate::gfx::atomic_age::draw(world);')
hook('depth','horizon_client_render::gfx::native_render::native_render_needs',[('needs','&mut crate::gfx::native_render::RenderNeeds','needs')],'needs.world_depth = true;')
(B/'horizon-mixin.toml').write_text(manifest)
(ROOT/'horizon-native.json').write_text(json.dumps(dict(schema=1,id='atomic_age',version='0.5.0',game='26.3',horizon='fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca',mixins='5a64af938ec86b00d53d16a408aabc0038fb3e57'),indent=2)+'\n')

# Restore renderer preferences on every normal/error return from the outer frame.
hook('finish','horizon_client_render::gfx::Renderer::render',[('renderer','&mut crate::gfx::Renderer','&mut *self'),('result','Result<bool>','$value')],'crate::gfx::atomic_age::finish(renderer); result','RETURN','Result<bool>')
(B/'horizon-mixin.toml').write_text(manifest)
hook('extract_begin',"horizon_client::game::Game<'_>::entity_geometry",[g],'crate::game::atomic_age::extract_begin(game);')
geometry='(Vec<Vertex>, Arc<[StaticSectionSnapshot]>, Vec<[i32; 3]>, StaticCacheFrameStats,)'
hook('extract_end',"horizon_client::game::Game<'_>::entity_geometry",[g,('value',geometry,'$value')],'crate::game::atomic_age::extract_end(game); value','RETURN',geometry)
hook('cpu_models','horizon_client_render::gfx::Renderer::skins_entities',[('value','bool','$value')],'value && !crate::gfx::atomic_age::has_portals()','RETURN','bool')
(B/'horizon-mixin.toml').write_text(manifest)
