"""Replay production renderer calls on an offscreen ANGLE framebuffer.

The JSON is captured by headless-world-capture.mjs from the actual WASM world,
camera and JS loaders/renderers. No app, browser, native executable or UI opens.
"""
from __future__ import annotations
import argparse
import base64
import ctypes as C
import hashlib
import json
from pathlib import Path
from PIL import Image
from headless_gles import HeadlessGLES, I, U, F, P

def replay(source, destination, report_path=None):
    raw=Path(source).read_bytes()
    payload=json.loads(raw)
    if payload.get('format')!='openepoch-production-world-capture-v1':
        raise ValueError('Unsupported world capture format')
    width,height=payload['width'],payload['height']
    programs,buffers,textures,framebuffers,renderbuffers={},{},{},{},{}
    programs_report=[]
    stats={'drawCalls':0,'instancedCalls':0,'drawnInstances':0,'triangles':0,'bufferBytes':0,'textureBytes':0}
    with HeadlessGLES(size=max(width,height)) as ctx:
        gl=ctx.gl
        gen_buffers=ctx.bind('glGenBuffers',None,I,C.POINTER(U))
        bind_buffer=ctx.bind('glBindBuffer',None,U,U)
        buffer_data=ctx.bind('glBufferData',None,U,C.c_ssize_t,P,U)
        gen_textures=ctx.bind('glGenTextures',None,I,C.POINTER(U))
        active_texture=ctx.bind('glActiveTexture',None,U)
        bind_texture=ctx.bind('glBindTexture',None,U,U)
        tex_image=ctx.bind('glTexImage2D',None,U,I,I,I,I,I,U,U,P)
        tex_image3=ctx.bind('glTexImage3D',None,U,I,I,I,I,I,I,U,U,P)
        tex_param=ctx.bind('glTexParameteri',None,U,U,I)
        mipmap=ctx.bind('glGenerateMipmap',None,U)
        ctx.bind('glPixelStorei',None,U,I)(0x0cf5,1)
        gen_vao=ctx.bind('glGenVertexArrays',None,I,C.POINTER(U))
        bind_vao=ctx.bind('glBindVertexArray',None,U)
        enable_attribute=ctx.bind('glEnableVertexAttribArray',None,U)
        pointer=ctx.bind('glVertexAttribPointer',None,U,I,U,C.c_ubyte,I,P)
        divisor=ctx.bind('glVertexAttribDivisor',None,U,U)
        uniform_location=ctx.bind('glGetUniformLocation',I,U,C.c_char_p)
        uniform_i=ctx.bind('glUniform1i',None,I,I)
        uniform_f=ctx.bind('glUniform1f',None,I,F)
        uniform_vectors={n:ctx.bind(f'glUniform{n}fv',None,I,I,C.POINTER(F)) for n in (2,3,4)}
        uniform_matrix=ctx.bind('glUniformMatrix4fv',None,I,I,C.c_ubyte,C.POINTER(F))
        use=ctx.bind('glUseProgram',None,U)
        enable=ctx.bind('glEnable',None,U)
        disable=ctx.bind('glDisable',None,U)
        depth_mask=ctx.bind('glDepthMask',None,C.c_ubyte)
        depth_func=ctx.bind('glDepthFunc',None,U)
        blend=ctx.bind('glBlendFuncSeparate',None,U,U,U,U)
        equation=ctx.bind('glBlendEquationSeparate',None,U,U)
        stencil_mask=ctx.bind('glStencilMask',None,U)
        stencil_func=ctx.bind('glStencilFunc',None,U,I,U)
        stencil_op=ctx.bind('glStencilOp',None,U,U,U)
        draw_elements=ctx.bind('glDrawElementsInstanced',None,U,I,U,P,I)
        draw_arrays=ctx.bind('glDrawArraysInstanced',None,U,I,I,I)
        gen_framebuffers=ctx.bind('glGenFramebuffers',None,I,C.POINTER(U))
        bind_framebuffer=ctx.bind('glBindFramebuffer',None,U,U)
        attach_texture=ctx.bind('glFramebufferTexture2D',None,U,U,U,U,I)
        gen_renderbuffers=ctx.bind('glGenRenderbuffers',None,I,C.POINTER(U))
        bind_renderbuffer=ctx.bind('glBindRenderbuffer',None,U,U)
        renderbuffer_storage=ctx.bind('glRenderbufferStorage',None,U,U,I,I)
        attach_renderbuffer=ctx.bind('glFramebufferRenderbuffer',None,U,U,U,U)
        framebuffer_status=ctx.bind('glCheckFramebufferStatus',U,U)
        viewport=ctx.bind('glViewport',None,I,I,I,I)
        color_mask=ctx.bind('glColorMask',None,C.c_ubyte,C.c_ubyte,C.c_ubyte,C.c_ubyte)
        front_face=ctx.bind('glFrontFace',None,U)
        cull_face=ctx.bind('glCullFace',None,U)
        ctx.bind('glViewport',None,I,I,I,I)(0,0,width,height)
        ctx.bind('glClearColor',None,F,F,F,F)(.012,.03,.05,1)
        ctx.bind('glClearDepthf',None,F)(1)
        ctx.bind('glClearStencil',None,I)(0)
        depth_mask(1)
        ctx.bind('glClear',None,U)(0x4000|0x100|0x400)

        for key,row in payload['programs'].items():
            programs[key]=ctx.shader_program(row['vertex'],row['fragment'])
            shader_hashes={
                'vertexSha256':hashlib.sha256(row['vertex'].encode()).hexdigest(),
                'fragmentSha256':hashlib.sha256(row['fragment'].encode()).hexdigest()}
            expected=payload.get('sourceHashes',{}).get('shaders',{}).get(key)
            if expected is not None and expected!=shader_hashes:
                raise ValueError('Captured shader source hashes do not match production metadata')
            programs_report.append({'id':key,'compiled':True,'linked':True,**shader_hashes})

        def get_buffer(key,target):
            if key not in buffers:
                data=base64.b64decode(payload['buffers'][key]['base64'])
                handle=U();gen_buffers(1,C.byref(handle));bind_buffer(target,handle)
                memory=C.create_string_buffer(data)
                buffer_data(target,len(data),memory,0x88e4)
                buffers[key]=handle.value;stats['bufferBytes']+=len(data)
            bind_buffer(target,buffers[key])

        def get_texture(key):
            key=str(key)
            if key in textures:
                return textures[key]
            row=payload['textures'][key]
            target=row.get('target')
            if target not in (0x0de1,0x8c1a):
                raise ValueError(f'Unsupported texture target {target} for{key}')
            handle=U();gen_textures(1,C.byref(handle));bind_texture(target,handle)
            if row.get('images'):
                layers=[]
                for layer in range(row['depth']):
                    image_row=row['images'].get(str(layer))
                    if image_row is None:
                        raise ValueError('Incomplete captured native monthly texture array')
                    original=Image.open(image_row['image']).convert('RGBA')
                    if original.size!=(row['width'],row['height']):
                        raise ValueError('Captured monthly texture dimensions changed')
                    layers.append(original.tobytes())
                data=b''.join(layers)
            elif row.get('image'):
                original=Image.open(row['image']).convert('RGBA')
                if original.size!=(row['width'],row['height']):
                    raise ValueError('Captured native PNG dimensions changed')
                data=original.tobytes()
            elif row.get('data'):
                data=base64.b64decode(row['data']['base64'])
            else:
                data=None
            memory=C.create_string_buffer(data) if data is not None else None
            internal=row['internal'];fmt=row.get('format',0x1908);kind=row.get('type',0x1406 if internal==0x8814 else 0x1401)
            if target==0x0de1:
                tex_image(target,0,internal,row['width'],row['height'],0,fmt,kind,memory)
            else:
                tex_image3(target,0,internal,row['width'],row['height'],row['depth'],0,fmt,kind,memory)
            for enum,value in row.get('params',{}).items():
                tex_param(target,int(enum),value)
            if row.get('mipmap'):
                mipmap(target)
            textures[key]=handle.value
            stats['textureBytes']+=len(data) if data is not None else 0
            ctx.check('Captured texture upload')
            return handle.value

        def get_framebuffer(key):
            if key is None:
                return ctx.framebuffer.value
            key=str(key)
            if key not in framebuffers:
                row=payload['framebuffers'][key]
                handle=U();gen_framebuffers(1,C.byref(handle));bind_framebuffer(0x8d40,handle)
                for attachment,binding in row['attachments'].items():
                    if 'texture' in binding:
                        texture=get_texture(binding['texture'])
                        attach_texture(0x8d40,int(attachment),binding['target'],texture,binding['level'])
                    else:
                        identifier=str(binding['renderbuffer'])
                        if identifier not in renderbuffers:
                            data=payload['renderbuffers'][identifier]
                            depth=U();gen_renderbuffers(1,C.byref(depth));bind_renderbuffer(0x8d41,depth)
                            renderbuffer_storage(0x8d41,data['internal'],data['width'],data['height']);renderbuffers[identifier]=depth.value
                        attach_renderbuffer(0x8d40,int(attachment),0x8d41,renderbuffers[identifier])
                if framebuffer_status(0x8d40)!=0x8cd5:
                    raise AssertionError('Captured production framebuffer is incomplete')
                framebuffers[key]=handle.value
            return framebuffers[key]

        commands=payload.get('commands') or [{'draw':number} for number in range(len(payload['draws']))]
        for command in commands:
            if 'clear' in command:
                bind_framebuffer(0x8d40,get_framebuffer(command.get('framebuffer')))
                disable(0x0c11);viewport(*command['viewport']);color_mask(*command['colorMask']);depth_mask(command['depthMask']);stencil_mask(command['stencilMask'])
                ctx.bind('glClearColor',None,F,F,F,F)(*command['color']);ctx.bind('glClearDepthf',None,F)(command['depth']);ctx.bind('glClearStencil',None,I)(command['stencil'])
                ctx.bind('glClear',None,U)(command['clear']);ctx.check('Captured production framebuffer clear');continue
            number=command['draw'];call=payload['draws'][number]
            bind_framebuffer(0x8d40,get_framebuffer(call.get('framebuffer')))
            program=programs[str(call['program'])];use(program)
            vao=U();gen_vao(1,C.byref(vao));bind_vao(vao)
            for loc,attribute in call['attributes'].items():
                if not attribute.get('enabled'):
                    continue
                get_buffer(attribute['buffer'],0x8892)
                enable_attribute(int(loc))
                pointer(int(loc),attribute['size'],attribute['type'],bool(attribute.get('normalized',False)),attribute['stride'],P(attribute['offset']))
                divisor(int(loc),attribute.get('divisor',0))
            if call.get('element'):
                get_buffer(call['element'],0x8893)
            for binding,key in call['textures'].items():
                unit,target=[int(x) for x in binding.split(':')]
                active_texture(0x84c0+unit)
                value=get_texture(key)
                bind_texture(target,value)
            for name,row in call['uniforms'].items():
                loc=uniform_location(program,name.encode());kind,value=row['kind'],row['value']
                if kind=='1i':uniform_i(loc,int(value))
                elif kind=='1f':uniform_f(loc,value)
                elif kind=='matrix4':uniform_matrix(loc,len(value)//16,0,(F*len(value))(*value))
                elif kind in ('2fv','3fv','4fv'):
                    n=int(kind[0]);uniform_vectors[n](loc,len(value)//n,(F*len(value))(*value))
                else:raise ValueError(f'Unsupported captured uniform kind {kind}')
            state=call['state']
            viewport(*state.get('viewport',[0,0,width,height]));color_mask(*state.get('colorMask',[True]*4))
            front_face(state.get('frontFace',0x901));cull_face(state.get('cullFace',0x405))
            for cap in (0x0be2,0x0b71,0x0b90,0x0b44):
                (enable if cap in state['enabled'] else disable)(cap)
            depth_mask(bool(state['depthWrite']));depth_func(state['depthFunc'])
            blend(*state['blend']);equation(*state['equation'])
            if state.get('stencil') is not None:
                stencil=state['stencil']
                stencil_mask(stencil['mask'])
                stencil_func(*stencil['func'])
                stencil_op(*stencil['op'])
            ctx.check(f'Captured draw{number} setup')
            if call.get('element'):
                draw_elements(call['mode'],call['count'],call['type'],P(call['offset']),call['instances'])
            else:
                draw_arrays(call['mode'],call['offset'],call['count'],call['instances'])
            ctx.check(f'Captured draw{number}')
            stats['drawCalls']+=1;stats['instancedCalls']+=int(call['instances']>1);stats['drawnInstances']+=call['instances']
            stats['triangles']+=(call['count']//3 if call['mode']==4 else max(0,call['count']-2))*call['instances']
        ctx.bind('glFinish',None)()
        reflection_outputs=[]
        for key,handle in framebuffers.items():
            attachments=payload['framebuffers'][key]['attachments']
            color=attachments.get(str(0x8ce0))
            if not color or 'texture' not in color:
                continue
            texture_row=payload['textures'][str(color['texture'])];fw,fh=texture_row['width'],texture_row['height']
            bind_framebuffer(0x8d40,handle);pixels=(C.c_ubyte*(fw*fh*4))()
            ctx.bind('glReadPixels',None,I,I,I,I,U,U,P)(0,0,fw,fh,0x1908,0x1401,pixels);ctx.check('Production reflection framebuffer readback')
            output=Path(destination).with_name(Path(destination).stem+f'-reflection-{key}.png')
            output.parent.mkdir(parents=True,exist_ok=True)
            Image.frombytes('RGBA',(fw,fh),bytes(pixels)).transpose(Image.Transpose.FLIP_TOP_BOTTOM).convert('RGB').save(output)
            reflection_outputs.append({'framebuffer':key,'texture':color['texture'],'size':[fw,fh],'output':str(output.resolve()),'pngSha256':hashlib.sha256(output.read_bytes()).hexdigest()})
        bind_framebuffer(0x8d40,ctx.framebuffer.value)
        pixels=(C.c_ubyte*(width*height*4))()
        ctx.bind('glReadPixels',None,I,I,I,I,U,U,P)(0,0,width,height,0x1908,0x1401,pixels)
        ctx.check('World RGBA readback')
        image=Image.frombytes('RGBA',(width,height),bytes(pixels)).transpose(Image.Transpose.FLIP_TOP_BOTTOM)
        destination=Path(destination);destination.parent.mkdir(parents=True,exist_ok=True)
        image.convert('RGB').save(destination)
        rgb=image.convert('RGB');hist=rgb.histogram()
        variations=sum(1 for count in hist if count)
        if variations<30:
            raise AssertionError('World replay produced insufficient color variation')
        report={'format':'openepoch-production-world-gles-replay-v1','passed':True,'context':ctx.info,
            'capture':str(Path(source).resolve()),'captureSha256':hashlib.sha256(raw).hexdigest(),
            'sourceEpoch':payload['epoch'],'scene':payload['scene'],'wasmInstances':payload['instances'],
            'size':[width,height],'stats':stats,'compiledPrograms':programs_report,
            'output':str(destination.resolve()),'pngSha256':hashlib.sha256(destination.read_bytes()).hexdigest(),
            'colorHistogramBins':variations,'scope':payload['scope'],'uiLaunched':False,
            'reflectionTargets':reflection_outputs,
            'limitation':'GPU replay of captured production world passes; not a browser/full HUD interaction test or a claim of original-game visual equality.'}
        for name in ('sourceHashes','season','ground','epochs','reflection','modelSeason'):
            if name in payload:
                report[name]=payload[name]
    if report_path:
        Path(report_path).write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
    return report

if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('capture',type=Path)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--report',type=Path)
    args=parser.parse_args()
    print(json.dumps(replay(args.capture,args.output,args.report),indent=2))
