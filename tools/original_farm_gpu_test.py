"""Actual GLES transform-feedback tests of the unchanged production model VS.

Reads vPosition/vNormal from the GPU, using non-coplanar terrain cells. No UI,
browser, original game process, or JS approximation of the shader is executed.
"""
import argparse,ctypes as C,hashlib,json,math,pathlib,struct
from headless_gles import HeadlessGLES,I,U,F,P

def run(payload,report_path):
    shader=payload['programs'][0];checks=[]
    with HeadlessGLES(32)as ctx:
        program=ctx.shader_program(shader['vertex'],shader['fragment'])
        names=(C.c_char_p*2)(b'vPosition',b'vNormal')
        ctx.bind('glTransformFeedbackVaryings',None,U,I,C.POINTER(C.c_char_p),U)(program,2,names,0x8c8c)
        ctx.bind('glLinkProgram',None,U)(program);ok=I();ctx.bind('glGetProgramiv',None,U,U,C.POINTER(I))(program,0x8b82,C.byref(ok));assert ok.value
        ctx.bind('glUseProgram',None,U)(program)
        vao=U();ctx.bind('glGenVertexArrays',None,I,C.POINTER(U))(1,C.byref(vao));ctx.bind('glBindVertexArray',None,U)(vao)
        location=ctx.bind('glGetUniformLocation',I,U,C.c_char_p)
        def uniform(name,value):
            at=location(program,name.encode())
            if isinstance(value,int):ctx.bind('glUniform1i',None,I,I)(at,value)
            elif isinstance(value,float):ctx.bind('glUniform1f',None,I,F)(at,value)
            else:ctx.bind('glUniform3fv',None,I,I,C.POINTER(F))(at,1,(F*3)(*value))
        attribute=ctx.bind('glVertexAttrib4f',None,U,F,F,F,F)
        buffer=U();ctx.bind('glGenBuffers',None,I,C.POINTER(U))(1,C.byref(buffer))
        ctx.bind('glBindBuffer',None,U,U)(0x8c8e,buffer);ctx.bind('glBufferData',None,U,C.c_ssize_t,P,U)(0x8c8e,24,None,0x88e5)
        ctx.bind('glBindBufferBase',None,U,U,U)(0x8c8e,0,buffer)
        ctx.bind('glEnable',None,U)(0x8c89)
        uniform('uAnimated',0);uniform('uShadow',0);uniform('uYOffset',0.);uniform('uOriginOffset',[0,0,0]);uniform('uTerrainSize',5);uniform('uTerrain',6);uniform('uTerrainEnabled',1)
        # All active sampler types must have distinct units even under discard.
        for name,value in [('uFrames',5),('uDiffuse',0),('uDiffuseNext',7),('uFog',2)]:uniform(name,value)
        texture=U();ctx.bind('glGenTextures',None,I,C.POINTER(U))(1,C.byref(texture));ctx.bind('glActiveTexture',None,U)(0x84c6);ctx.bind('glBindTexture',None,U,U)(0xde1,texture)
        params=ctx.bind('glTexParameteri',None,U,U,I)
        for name,value in [(0x2801,0x2600),(0x2800,0x2600),(0x2802,0x812f),(0x2803,0x812f)]:params(0xde1,name,value)
        heights=[0,2,1,5,1, 3,8,4,9,0, 2,5,14,6,2, 1,10,3,11,1, 3,0,2,1,0]
        ctx.bind('glTexImage2D',None,U,I,I,I,I,I,U,U,P)(0xde1,0,0x822e,5,5,0,0x1903,0x1406,(F*25)(*heights))
        def ground(x,z):
            x=max(0,min(3.9999,x));z=max(0,min(3.9999,z));ix=int(x);iz=int(z);fx=x-ix;fz=z-iz
            a,b,c,d=[heights[k]for k in [iz*5+ix,iz*5+ix+1,(iz+1)*5+ix,(iz+1)*5+ix+1]]
            return a+(b-a)*fx+(c-a)*fz if fx+fz<=1 else d+(c-d)*(1-fx)+(b-d)*(1-fz)
        def read():
            ctx.bind('glBeginTransformFeedback',None,U)(0);ctx.bind('glDrawArrays',None,U,I,I)(0,0,1);ctx.bind('glEndTransformFeedback',None)()
            ctx.bind('glBindBuffer',None,U,U)(0x8c8e,buffer)
            memory=ctx.bind('glMapBufferRange',P,U,C.c_ssize_t,C.c_ssize_t,U)(0x8c8e,0,24,1)
            assert memory;result=struct.unpack('<6f',C.string_at(memory,24));assert ctx.bind('glUnmapBuffer',C.c_ubyte,U)(0x8c8e)
            ctx.check('Native FarmPlot transform feedback');return result
        attribute(1,0,1,0,0);attribute(2,0,0,0,1);attribute(4,2,0,1,96);attribute(5,2,0,0,0);attribute(6,1,1,1,1)
        for yaw in [k*math.pi/4 for k in range(8)]:
            cy=3.25;c=math.cos(yaw);s=math.sin(yaw);attribute(3,2,2,cy,yaw)
            for center in [(-.25,0,.25),(.25,0,.25),(.25,0,-.25),(-.25,0,-.25)]:
                # Independent center-height translation must leave plot corners
                # vertical, rather than warp each vertex along the slope.
                for delta in [(0,0,0),(.12,.08,-.09)]:
                    point=[center[k]+delta[k]for k in range(3)];attribute(0,*point,1)
                    uniform('uFarmPlotCenter',center);uniform('uFarmPlotConform',1)
                    result=read();cx=2+2*(center[0]*c-center[2]*s);cz=2+2*(center[0]*s+center[2]*c)
                    expected=[2+2*(point[0]*c-point[2]*s),ground(cx,cz)+2*delta[1],2+2*(point[0]*s+point[2]*c),0,1,0]
                    assert max(abs(a-b)for a,b in zip(result,expected))<7e-6,(result,expected)
                    checks.append({'yaw':yaw,'plotCenter':center,'corner':delta,'gpu':result,'expected':expected})
        uniform('uFarmPlotConform',0);attribute(0,.25,.1,-.25,1);attribute(3,2,2,3.25,0)
        result=read();assert max(abs(a-b)for a,b in zip(result,[2.5,3.45,1.5,0,1,0]))<1e-6;checks.append({'case':'non-Farm path preserves original instance height','gpu':result})
        uniform('uFarmPlotConform',1);uniform('uTerrainEnabled',0);result=read();assert abs(result[1]-3.45)<1e-6;checks.append({'case':'missing terrain keeps compatible model path','gpu':result})
        report={'status':'passed','build':'water-farm3','format':'ee2-native-farm-gpu-v1','checks':len(checks),'cases':checks,'context':ctx.info,'vertexSha256':hashlib.sha256(shader['vertex'].encode()).hexdigest(),'fragmentSha256':hashlib.sha256(shader['fragment'].encode()).hexdigest(),'uiLaunched':False,'limits':'Production vertex shader on native GLES; actual Farm source-mesh/material scene replay is a separate test. Terrain interpolation adapts the native center-height query to the current terrain triangles.'}
    pathlib.Path(report_path).write_text(json.dumps(report,indent=2)+'\n');return report

if __name__=='__main__':
    ap=argparse.ArgumentParser();ap.add_argument('input',type=pathlib.Path);ap.add_argument('--report',required=True,type=pathlib.Path);args=ap.parse_args()
    result=run(json.loads(args.input.read_text()),args.report);print(json.dumps({key:result[key]for key in ['status','checks','context']}))
