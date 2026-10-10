function n = refractive_index(material, wavelength_um, options)
%REFRACTIVE_INDEX  The refractive index n + i*kappa of a material of photonoxide's catalogue.
%   n = photonoxide.refractive_index(material, wavelength_um) is the index of the catalogue's
%   material (an id: "si", "sio2", "linbo3", ...; photonoxide.materials lists them) at the
%   vacuum wavelengths wavelength_um, in micrometres; n has wavelength_um's size.
%
%   n = photonoxide.refractive_index(..., model=id, axis=name, temperature_k=t, composition=x)
%   picks an index model other than the default, an axis ("ordinary" or "extraordinary") for a
%   material with two, and a model's conditions.
%
%   A wavelength outside the model's data is the error photonoxide:OutsideValidity: photonoxide
%   doesn't extrapolate.
arguments
    material (1,1) string
    wavelength_um double
    options.model (1,1) string
    options.axis (1,1) string
    options.temperature_k (1,1) double
    options.composition (1,1) double
end
kw = namedargs2cell(options);
r = call(@() py.photonoxide.matlab.refractive_index(material, pylist(wavelength_um), pyargs(kw{:})));
n = reshape(unflat(r), size(wavelength_um));
end
