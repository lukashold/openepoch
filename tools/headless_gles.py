"""Headless GLES 3 shader checks using installed ANGLE DLLs; no app is launched.

Usage: python headless_gles.py shaders.json --report report.json
Input: {"programs": [{"name": "terrain", "vertex": "...", "fragment": "..."}]}
Reusable HeadlessGLES exposes shader_program(), gl (DLL), bind(), and check().
The caller owns any draw buffers/textures it creates. Pixel readback uses rgba().
"""
from __future__ import annotations
import argparse
import ctypes as C
import hashlib
import json
import os
from pathlib import Path

I, U, F, P = C.c_int, C.c_uint, C.c_float, C.c_void_p

class HeadlessGLES:
    def __init__(self, size=64, dll_dir=None):
        candidates = [Path(dll_dir)] if dll_dir else [Path(r'C:\Program Files\Mozilla Firefox'), *Path(r'C:\Program Files\Microsoft VS Code').glob('*')]
        self.directory = next((p for p in candidates if (p / 'libEGL.dll').is_file() and (p / 'libGLESv2.dll').is_file()), None)
        if self.directory is None:
            raise RuntimeError('No installed ANGLE EGL/GLES libraries found')
        self.search = os.add_dll_directory(str(self.directory))
        self.egl = C.WinDLL(str(self.directory / 'libEGL.dll'))
        self.gl = C.WinDLL(str(self.directory / 'libGLESv2.dll'))
        self.functions = {}
        self.display = self.surface = self.context = None
        self.size = size
        self._egl('eglGetDisplay', P, P)
        self._egl('eglInitialize', U, P, C.POINTER(I), C.POINTER(I))
        self._egl('eglGetError', I)
        self._egl('eglChooseConfig', U, P, C.POINTER(I), C.POINTER(P), I, C.POINTER(I))
        self._egl('eglBindAPI', U, U)
        self._egl('eglCreatePbufferSurface', P, P, P, C.POINTER(I))
        self._egl('eglCreateContext', P, P, P, P, C.POINTER(I))
        self._egl('eglMakeCurrent', U, P, P, P, P)
        self._egl('eglDestroyContext', U, P, P)
        self._egl('eglDestroySurface', U, P, P)
        self._egl('eglTerminate', U, P)
        self.display = self.egl.eglGetDisplay(None)
        major, minor = I(), I()
        if not self.display or not self.egl.eglInitialize(self.display, C.byref(major), C.byref(minor)):
            raise RuntimeError(f'eglInitialize failed 0x{self.egl.eglGetError():04x}')
        self.egl_version = [major.value, minor.value]
        config, count = P(), I()
        attrs = (I * 17)(0x3033, 1, 0x3040, 0x40, 0x3024, 8, 0x3023, 8, 0x3022, 8, 0x3021, 8, 0x3025, 24, 0x3026, 8, 0x3038)
        if not self.egl.eglChooseConfig(self.display, attrs, C.byref(config), 1, C.byref(count)) or not count.value:
            raise RuntimeError(f'No GLES3 pbuffer config 0x{self.egl.eglGetError():04x}')
        if not self.egl.eglBindAPI(0x30a0):
            raise RuntimeError('Cannot bind EGL_OPENGL_ES_API')
        pbuffer_attrs = (I * 5)(0x3057, size, 0x3056, size, 0x3038)
        self.surface = self.egl.eglCreatePbufferSurface(self.display, config, pbuffer_attrs)
        context_attrs = (I * 3)(0x3098, 3, 0x3038)
        self.context = self.egl.eglCreateContext(self.display, config, None, context_attrs)
        if not self.surface or not self.context or not self.egl.eglMakeCurrent(self.display, self.surface, self.surface, self.context):
            raise RuntimeError(f'GLES3 pbuffer context failed 0x{self.egl.eglGetError():04x}')
        self.bind('glGetString', C.c_char_p, U)
        self.info = {'egl': self.egl_version, 'vendor': self.gl.glGetString(0x1f00).decode(),
            'renderer': self.gl.glGetString(0x1f01).decode(), 'version': self.gl.glGetString(0x1f02).decode(),
            'libraryDirectory': str(self.directory), 'surface': [size, size], 'uiLaunched': False}
        # Use a texture-backed framebuffer rather than relying on implementation-
        # specific default-pbuffer readback behavior in bundled ANGLE builds.
        self.color_texture, self.framebuffer, self.depth_stencil = U(), U(), U()
        self.bind('glGenTextures', None, I, C.POINTER(U))(1, C.byref(self.color_texture))
        self.bind('glBindTexture', None, U, U)(0x0de1, self.color_texture)
        self.bind('glTexImage2D', None, U, I, I, I, I, I, U, U, P)(0x0de1, 0, 0x8058, size, size, 0, 0x1908, 0x1401, None)
        parameter = self.bind('glTexParameteri', None, U, U, I)
        parameter(0x0de1, 0x2801, 0x2600); parameter(0x0de1, 0x2800, 0x2600)
        self.bind('glGenFramebuffers', None, I, C.POINTER(U))(1, C.byref(self.framebuffer))
        self.bind('glBindFramebuffer', None, U, U)(0x8d40, self.framebuffer)
        self.bind('glFramebufferTexture2D', None, U, U, U, U, I)(0x8d40, 0x8ce0, 0x0de1, self.color_texture, 0)
        self.bind('glGenRenderbuffers', None, I, C.POINTER(U))(1, C.byref(self.depth_stencil))
        self.bind('glBindRenderbuffer', None, U, U)(0x8d41, self.depth_stencil)
        self.bind('glRenderbufferStorage', None, U, U, I, I)(0x8d41, 0x88f0, size, size)
        self.bind('glFramebufferRenderbuffer', None, U, U, U, U)(0x8d40, 0x821a, 0x8d41, self.depth_stencil)
        status = self.bind('glCheckFramebufferStatus', U, U)(0x8d40)
        if status != 0x8cd5:
            raise RuntimeError(f'Incomplete headless framebuffer 0x{status:04x}')
        self.bind('glReadBuffer', None, U)(0x8ce0)
        self.check('Headless RGBA8/depth-stencil framebuffer')
        self.info['framebuffer'] = 'RGBA8 texture + DEPTH24_STENCIL8 renderbuffer'

    def _egl(self, name, restype, *argtypes):
        f = getattr(self.egl, name)
        f.restype, f.argtypes = restype, list(argtypes)
        return f

    def bind(self, name, restype, *argtypes):
        if name not in self.functions:
            f = getattr(self.gl, name)
            f.restype, f.argtypes = restype, list(argtypes)
            self.functions[name] = f
        return self.functions[name]

    def check(self, context='GL operation'):
        err = self.bind('glGetError', U)()
        if err:
            raise AssertionError(f'{context}: GL error 0x{err:04x}')

    def shader_program(self, vertex, fragment):
        create_shader = self.bind('glCreateShader', U, U)
        source = self.bind('glShaderSource', None, U, I, C.POINTER(C.c_char_p), C.POINTER(I))
        compile_shader = self.bind('glCompileShader', None, U)
        shader_iv = self.bind('glGetShaderiv', None, U, U, C.POINTER(I))
        shader_log = self.bind('glGetShaderInfoLog', None, U, I, C.POINTER(I), C.c_char_p)
        create_program = self.bind('glCreateProgram', U)
        attach = self.bind('glAttachShader', None, U, U)
        link = self.bind('glLinkProgram', None, U)
        program_iv = self.bind('glGetProgramiv', None, U, U, C.POINTER(I))
        program_log = self.bind('glGetProgramInfoLog', None, U, I, C.POINTER(I), C.c_char_p)
        delete_shader = self.bind('glDeleteShader', None, U)
        shaders = []
        for kind, text in ((0x8b31, vertex), (0x8b30, fragment)):
            shader = create_shader(kind)
            encoded = text.encode('utf8')
            ptr = C.c_char_p(encoded)
            source(shader, 1, C.byref(ptr), None)
            compile_shader(shader)
            ok = I()
            shader_iv(shader, 0x8b81, C.byref(ok))
            if not ok.value:
                buffer = C.create_string_buffer(32768)
                shader_log(shader, len(buffer), None, buffer)
                raise AssertionError(f'{"vertex" if kind == 0x8b31 else "fragment"}: {buffer.value.decode()}')
            shaders.append(shader)
        program = create_program()
        for shader in shaders:
            attach(program, shader)
        link(program)
        ok = I()
        program_iv(program, 0x8b82, C.byref(ok))
        for shader in shaders:
            delete_shader(shader)
        if not ok.value:
            buffer = C.create_string_buffer(32768)
            program_log(program, len(buffer), None, buffer)
            raise AssertionError('link: ' + buffer.value.decode())
        self.check('shader compile/link')
        return program

    def rgba(self):
        self.bind('glFinish', None)()
        pixels = (C.c_ubyte * (self.size * self.size * 4))()
        self.bind('glReadPixels', None, I, I, I, I, U, U, P)(0, 0, self.size, self.size, 0x1908, 0x1401, pixels)
        self.check('pixel readback')
        return bytes(pixels)

    def close(self):
        if self.display:
            self.egl.eglMakeCurrent(self.display, None, None, None)
            if self.context:
                self.egl.eglDestroyContext(self.display, self.context)
            if self.surface:
                self.egl.eglDestroySurface(self.display, self.surface)
            self.egl.eglTerminate(self.display)
            self.display = None
        self.search.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('input', type=Path)
    parser.add_argument('--report', type=Path)
    args = parser.parse_args()
    payload = json.loads(args.input.read_text(encoding='utf-8-sig'))
    with HeadlessGLES() as ctx:
        programs = []
        for item in payload['programs']:
            program = ctx.shader_program(item['vertex'], item['fragment'])
            programs.append({'name': item['name'], 'compiled': True, 'linked': True,
                'vertexSha256': hashlib.sha256(item['vertex'].encode()).hexdigest(),
                'fragmentSha256': hashlib.sha256(item['fragment'].encode()).hexdigest()})
            ctx.bind('glDeleteProgram', None, U)(program)
        report = {'format': 'openepoch-headless-gles-programs-v1', 'context': ctx.info, 'programs': programs, 'uiLaunched': False}
    if args.report:
        args.report.write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
    print(json.dumps(report, indent=2))
