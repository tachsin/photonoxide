function m = slab_modes(below, core, above, thickness_um, polarization, wavelength_um, x_um)
%SLAB_MODES  The guided modes of a three-layer slab, solved exactly.
%   m = photonoxide.slab_modes(below, core, above, thickness_um, polarization, wavelength_um)
%   are the guided modes of one polarization ("te" or "tm") of a core of index core and
%   thickness thickness_um (micrometres) between indices below and above, at wavelength_um:
%   m.order and m.effective_index, a row each, fundamental first. Empty when it guides none.
%
%   m = photonoxide.slab_modes(..., x_um) adds m.field, their fields at x_um (0 at the core's
%   bottom; E_y for TE, H_y for TM, unnormalized), a column per mode.
arguments
    below (1,1) double
    core (1,1) double
    above (1,1) double
    thickness_um (1,1) double
    polarization (1,1) string
    wavelength_um (1,1) double
    x_um double = []
end
r = struct(call(@() py.photonoxide.matlab.slab_modes(below, core, above, thickness_um, ...
    polarization, wavelength_um, pylist(x_um))));
m.order = double(r.order);
m.effective_index = double(r.effective_index);
m.field = unflat(r.field);
end
