function s = fdfd_s_parameters(job, wavelength_um)
%FDFD_S_PARAMETERS  The S-parameters of an "fdfd" job's device by 2D FDFD.
%   s = photonoxide.fdfd_s_parameters(job) solves job (a job file's path, or a struct with its
%   fields) at its own wavelengths, and s = photonoxide.fdfd_s_parameters(job, wavelength_um)
%   at wavelength_um (micrometres). A 2D result, by the effective index method: an estimate, not
%   a device's performance in 3D.
%
%   s.ports are the ports' names; s.wavelength_um the wavelengths; s.s the power-normalized
%   S-matrices, nl-by-n-by-n, s.s(k, q, p) from port p into port q at the k-th wavelength;
%   s.effective_index each port mode's, nl-by-n; s.cell_um the grid's cell [dx dy]; and
%   s.polarization "te" or "tm".
arguments
    job
    wavelength_um double = []
end
r = struct(call(@() py.photonoxide.matlab.fdfd_s_parameters(jobtext(job), pylist(wavelength_um))));
s.ports = strings(r.ports);
s.wavelength_um = double(r.wavelength_um);
s.s = unflat(r.s);
s.effective_index = unflat(r.effective_index);
s.cell_um = double(r.cell_um);
s.polarization = string(r.polarization);
end
