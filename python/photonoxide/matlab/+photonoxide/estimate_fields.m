function f = estimate_fields(run_dir, options)
%ESTIMATE_FIELDS  The field files of a photonoxide run, their sizes estimated before writing.
%   f = photonoxide.estimate_fields(run_dir) lists the field files of the run in run_dir (its
%   .npz files, and a container's .h5), a struct array with each one's file, its bytes as the
%   run wrote it, its estimate written again as asked, whether that estimate is exact, the
%   precision its fields would have and whether it is rewritable (an .h5, or an .npz
%   compressed with Zstandard, is copied as it is). Stored, the estimate is the size to the
%   byte; deflated, it is for members up to 1 MiB, and from slices of larger ones.
%
%   Options: precision, "f64" or "f32" (the fields downcast, the grid kept f64), and level,
%   deflate's 0 (stored) to 9 (smallest).
arguments
    run_dir (1,1) string
    options.precision (1,1) string = "f64"
    options.level (1,1) double = 6
end
f = jsondecode(char(call(@() py.photonoxide.matlab.estimate_fields(run_dir, ...
    options.precision, options.level))));
end
