function ng = group_index(material, wavelength_um, options)
%GROUP_INDEX  The bulk group index n - lambda dn/dlambda of a material of photonoxide's catalogue.
%   ng = photonoxide.group_index(material, wavelength_um) at the vacuum wavelengths
%   wavelength_um, in micrometres; ng has wavelength_um's size. The options are
%   photonoxide.refractive_index's.
arguments
    material (1,1) string
    wavelength_um double
    options.model (1,1) string
    options.axis (1,1) string
    options.temperature_k (1,1) double
    options.composition (1,1) double
end
kw = namedargs2cell(options);
r = call(@() py.photonoxide.matlab.group_index(material, pylist(wavelength_um), pyargs(kw{:})));
ng = reshape(unflat(r), size(wavelength_um));
end
