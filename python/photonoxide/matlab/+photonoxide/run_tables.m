function t = run_tables(run_dir)
%RUN_TABLES  The tables of a photonoxide run, from its record.
%   t = photonoxide.run_tables(run_dir) reads the run in run_dir (run.dir of
%   photonoxide.run_job) and returns a struct of its tables: t.modes, t.sweep,
%   t.s_parameters, t.spectra, t.resonances, t.study, t.history, t.static, t.transient, those
%   the run has. Each is a MATLAB table whose variables are its CSV file's columns: numbers as
%   doubles (NaN where the table has none), text as strings. Every number is the one the run
%   recorded, to the bit.
arguments
    run_dir (1,1) string
end
r = jsondecode(char(call(@() py.photonoxide.matlab.run_tables(run_dir))));
t = struct();
for name = string(fieldnames(r))'
    columns = r.(name).columns;
    if ~iscell(columns)
        columns = num2cell(columns);
    end
    values = cell(1, numel(columns));
    names = cell(1, numel(columns));
    for k = 1:numel(columns)
        names{k} = columns{k}.name;
        v = columns{k}.values;
        if iscell(v)
            v = string(v);
        else
            v = double(v);
        end
        values{k} = v(:);
    end
    t.(name) = table(values{:}, 'VariableNames', names);
end
end
