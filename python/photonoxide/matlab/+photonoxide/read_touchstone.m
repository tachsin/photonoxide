function s = read_touchstone(file, convention)
%READ_TOUCHSTONE  The S-parameters of a Touchstone file.
%   s = photonoxide.read_touchstone(file, convention) reads file (Version 1 or 2.0), its values
%   in the time convention convention: "physics" (exp(-i w t), photonoxide's) or "engineering"
%   (exp(+j w t), most RF tools'). The format doesn't say which, so the reader must.
%
%   s.ports are "o1", "o2", ...; s.wavelength_um the wavelengths, c/f, in the file's order
%   (increasing frequency, so decreasing wavelength); s.s the S-matrices, nl-by-n-by-n, in
%   photonoxide's convention.
arguments
    file (1,1) string
    convention (1,1) string
end
r = struct(call(@() py.photonoxide.matlab.read_touchstone(file, convention)));
s.ports = strings(r.ports);
s.wavelength_um = double(r.wavelength_um);
s.s = unflat(r.s);
end
