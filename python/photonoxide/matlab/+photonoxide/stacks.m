function s = stacks()
%STACKS  The stacks of photonoxide's catalogue, by the name the device functions take.
%   s = photonoxide.stacks() is a struct array, one element per stack: s(k).name (what the
%   device functions take as stack, e.g. "soi_220"), s(k).about, s(k).substrate, s(k).layers (a
%   struct array, bottom to top: name, thickness_um, material, background), s(k).cladding and
%   s(k).core, the layer a device draws its guides on, as thick as the device's thickness. The
%   materials are the catalogue's ids, which photonoxide.refractive_index takes.
s = jsondecode(char(call(@() py.photonoxide.matlab.stacks())));
end
