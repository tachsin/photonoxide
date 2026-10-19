function s = device_spectrum(kind, wavelength_um, options)
%DEVICE_SPECTRUM  A device's S-parameters at a fidelity.
%   s = photonoxide.device_spectrum(kind, wavelength_um) computes the device kind of
%   photonoxide's library (see photonoxide.devices) at its defaults, at the vacuum wavelengths
%   wavelength_um (micrometres), by its cheapest model.
%
%   Options: values, a struct of its parameters' values in their units, e.g.
%   struct("gap", 0.25, "length", 10) (those not given at their defaults); fidelity
%   ("analytic", "2D", "3D", ... as the device has them); stack, one of photonoxide.stacks by
%   name ("soi_220", "soi" or "nitride"); compact, the file of a compact model fitted to its
%   runs (the compact.json a "fit" job writes), for fidelity "compact"; timeout_s (Inf: none;
%   past it, the error photonoxide:Timeout, the solve let go).
%
%   s.ports are the device's ports, s.wavelength_um the wavelengths, s.s the S-matrices,
%   nl-by-n-by-n: s.s(k, q, p) from port p into port q at the k-th wavelength; s.fidelity,
%   s.grid and s.source say where they come from, and s.validity_um the wavelengths the model
%   holds for (empty when not limited). These are the numbers the device's page in the
%   photonoxide studio shows for the same design.
arguments
    kind (1,1) string
    wavelength_um double
    options.values (1,1) struct = struct()
    options.fidelity (1,1) string = ""
    options.stack (1,1) string = ""
    options.compact (1,1) string = ""
    options.timeout_s (1,1) double = Inf
end
r = struct(call(@() py.photonoxide.matlab.device_spectrum(kind, jsonencode(options.values), ...
    options.fidelity, pylist(wavelength_um), options.stack, options.compact, options.timeout_s)));
s.ports = strings(r.ports);
s.wavelength_um = double(r.wavelength_um);
s.s = unflat(r.s);
s.fidelity = string(r.fidelity);
s.grid = string(r.grid);
s.source = string(r.source);
s.validity_um = double(r.validity_um);
end
