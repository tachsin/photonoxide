function m = materials()
%MATERIALS  The materials of photonoxide's catalogue.
%   m = photonoxide.materials() is a struct of string arrays, one element per material: m.id
%   (what photonoxide.refractive_index takes, e.g. "si"), m.name, m.formula, and of each one's
%   default index model m.model (its id), m.axes ("isotropic", or "ordinary,extraordinary"),
%   and m.range_um, the wavelengths it is valid for (2-by-n, micrometres).
r = struct(call(@() py.photonoxide.matlab.materials()));
m.id = strings(r.id);
m.name = strings(r.name);
m.formula = strings(r.formula);
m.model = strings(r.model);
m.axes = strings(r.axes);
m.range_um = unflat(r.range_um);
end
