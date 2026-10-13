// Decompiles every function and writes greppable text dumps:
//   <out>/code/<0xNNNN>.c   functions grouped by 64 KiB address bucket
//   <out>/functions.tsv     address, name, body size, caller count
//   <out>/calls.tsv         caller -> callee edges
//   <out>/strings.tsv       defined string address, referencing functions, value
//
// Script argument: absolute output directory (created if missing).
//@category FFX

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.decompiler.parallel.DecompileConfigurer;
import ghidra.app.decompiler.parallel.DecompilerCallback;
import ghidra.app.decompiler.parallel.ParallelDecompiler;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.data.StringDataInstance;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.DataIterator;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;
import ghidra.util.task.TaskMonitor;

import java.io.BufferedWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.TreeSet;

public class DumpDecomp extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) {
            throw new IllegalArgumentException("usage: DumpDecomp <output dir>");
        }
        Path out = Path.of(args[0]);
        Path code = out.resolve("code");
        Files.createDirectories(code);

        List<Function> functions = new ArrayList<>();
        for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
            if (!f.isExternal() && !f.isThunk()) {
                functions.add(f);
            }
        }
        println("DumpDecomp: decompiling " + functions.size() + " functions");

        DecompileConfigurer configurer = decompiler -> {
            DecompileOptions options = new DecompileOptions();
            options.grabFromProgram(currentProgram);
            decompiler.setOptions(options);
            decompiler.toggleCCode(true);
            decompiler.toggleSyntaxTree(false);
            decompiler.setSimplificationStyle("decompile");
        };
        DecompilerCallback<String[]> callback = new DecompilerCallback<>(currentProgram, configurer) {
            @Override
            public String[] process(DecompileResults results, TaskMonitor m) {
                Function f = results.getFunction();
                String text = results.decompileCompleted()
                        ? results.getDecompiledFunction().getC()
                        : "/* decompile failed: " + results.getErrorMessage() + " */\n";
                return new String[] { f.getEntryPoint().toString(), text };
            }
        };
        callback.setTimeout(120);
        List<String[]> decompiled;
        try {
            decompiled = ParallelDecompiler.decompileFunctions(callback, functions, monitor);
        } finally {
            callback.dispose();
        }

        // Results arrive in completion order; sort by address, then bucket
        // by 64 KiB so each file stays small enough to read.
        decompiled.removeIf(item -> item == null);
        decompiled.sort((a, b) -> Long.compare(Long.parseLong(a[0], 16), Long.parseLong(b[0], 16)));
        Map<String, StringBuilder> buckets = new TreeMap<>();
        for (String[] item : decompiled) {
            long addr = Long.parseLong(item[0], 16);
            String bucket = String.format("0x%08x", addr & ~0xffffL);
            buckets.computeIfAbsent(bucket, k -> new StringBuilder())
                    .append("// ==== 0x").append(item[0]).append(" ====\n")
                    .append(item[1]).append('\n');
        }
        for (Map.Entry<String, StringBuilder> e : buckets.entrySet()) {
            Files.writeString(code.resolve(e.getKey() + ".c"), e.getValue(), StandardCharsets.UTF_8);
        }

        try (BufferedWriter fw = Files.newBufferedWriter(out.resolve("functions.tsv"));
             BufferedWriter cw = Files.newBufferedWriter(out.resolve("calls.tsv"))) {
            fw.write("address\tname\tbody_bytes\tcallers\n");
            cw.write("caller\tcallee\tcallee_name\n");
            for (Function f : functions) {
                Set<Function> callers = f.getCallingFunctions(TaskMonitor.DUMMY);
                fw.write("0x" + f.getEntryPoint() + "\t" + f.getName() + "\t"
                        + f.getBody().getNumAddresses() + "\t" + callers.size() + "\n");
                for (Function callee : f.getCalledFunctions(TaskMonitor.DUMMY)) {
                    cw.write("0x" + f.getEntryPoint() + "\t0x" + callee.getEntryPoint() + "\t"
                            + callee.getName() + "\n");
                }
            }
        }

        try (BufferedWriter sw = Files.newBufferedWriter(out.resolve("strings.tsv"))) {
            sw.write("address\tfunctions\tvalue\n");
            DataIterator it = currentProgram.getListing().getDefinedData(true);
            while (it.hasNext()) {
                Data d = it.next();
                if (!StringDataInstance.isString(d)) {
                    continue;
                }
                Set<String> users = new TreeSet<>();
                for (Reference r : getReferencesTo(d.getAddress())) {
                    Function f = getFunctionContaining(r.getFromAddress());
                    users.add(f == null ? r.getFromAddress().toString() : "0x" + f.getEntryPoint());
                }
                String value = String.valueOf(d.getValue()).replace("\\", "\\\\")
                        .replace("\t", "\\t").replace("\n", "\\n").replace("\r", "\\r");
                sw.write("0x" + d.getAddress() + "\t" + String.join(",", users) + "\t" + value + "\n");
            }
        }
        println("DumpDecomp: wrote " + buckets.size() + " code files to " + out);
    }
}
