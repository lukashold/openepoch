"""Lossless material/state bridge from inherited NIF properties to runtime parts.

Numeric enum/flag values are retained. Decoding them in the renderer does not
imply that the complete original lighting or texture-stage pipeline is known.
"""
import copy

PROPERTY_TYPES = ('NiMaterialProperty', 'NiAlphaProperty', 'NiVertexColorProperty',
                  'NiZBufferProperty', 'NiSpecularProperty', 'NiStencilProperty')


def channels(prop):
    def rgb(key, fallback):
        value = prop.get(key)
        return [float(value.get(k, 0)) for k in 'rgb'] if value is not None else fallback
    return {
        'ambient': rgb('Ambient Color', [1., 1., 1.]),
        'diffuse': rgb('Diffuse Color', [1., 1., 1.]),
        'specular': rgb('Specular Color', [0., 0., 0.]),
        'emissive': rgb('Emissive Color', [0., 0., 0.]),
        'glossiness': float(prop.get('Glossiness', 0.)),
        'alpha': float(prop.get('Alpha', 1.)),
    }


def native_material_fields(properties):
    """Return source values, including absent properties and unmodified flags."""
    material = properties.get('NiMaterialProperty')
    return {
        'material': channels(material) if material is not None else None,
        'nativeProperties': {name: copy.deepcopy(properties.get(name)) for name in PROPERTY_TYPES},
        'materialStateVersion': 1,
    }
