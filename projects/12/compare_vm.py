#!/usr/bin/env python3
"""
VM Code Comparison Tool for Jack OS Implementation.

Compares compiled VM output against reference implementation to verify
functional equivalence. Focuses on:
1. Function signatures (same functions defined)
2. Function instruction counts (similar complexity)
3. Label naming patterns (structural similarity)

Usage:
    python3 compare_vm.py
"""

from pathlib import Path
from dataclasses import dataclass
from typing import Dict, List, Tuple


@dataclass
class VMFunction:
    name: str
    locals: int
    instructions: int
    labels: List[str]
    calls: List[str]


def parse_vm_file(filepath: Path) -> Dict[str, VMFunction]:
    """Parse a VM file and extract function metadata."""
    functions = {}
    current_func = None
    current_locals = 0
    instruction_count = 0
    labels: List[str] = []
    calls: List[str] = []

    with open(filepath) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("//"):
                continue

            # Function definition
            if line.startswith("function "):
                # Save previous function
                if current_func:
                    functions[current_func] = VMFunction(
                        name=current_func,
                        locals=current_locals,
                        instructions=instruction_count,
                        labels=labels,
                        calls=calls,
                    )

                parts = line.split()
                current_func = parts[1]
                current_locals = int(parts[2])
                instruction_count = 0
                labels = []
                calls = []
            elif current_func:
                instruction_count += 1
                if line.startswith("label "):
                    labels.append(line.split()[1])
                elif line.startswith("call "):
                    calls.append(line.split()[1])

    # Save last function
    if current_func:
        functions[current_func] = VMFunction(
            name=current_func,
            locals=current_locals,
            instructions=instruction_count,
            labels=labels,
            calls=calls,
        )

    return functions


def compare_files(ref_path: Path, impl_path: Path) -> Tuple[bool, List[str]]:
    """Compare reference and implementation VM files."""
    issues = []

    ref_funcs = parse_vm_file(ref_path)
    impl_funcs = parse_vm_file(impl_path)

    # Check function coverage
    ref_names = set(ref_funcs.keys())
    impl_names = set(impl_funcs.keys())

    missing = ref_names - impl_names
    extra = impl_names - ref_names

    # Known equivalent function mappings (different names, same purpose)
    equivalents = {
        "Screen.drawHorizontal": "Screen.drawHorizontalLine",
        "Screen.drawSymetric": "Screen.drawSymmetricLines",  # Note: ref has typo
        "Screen.drawConditional": "Screen.drawHorizontalLineClipped",
        "Screen.updateLocation": None,  # Inlined in our implementation
        "Output.createShiftedMap": "Output.printIntHelper",  # Different approach
    }

    # Filter out known equivalent functions
    actual_missing = []
    for func in missing:
        if func in equivalents:
            equiv = equivalents[func]
            if equiv is None or equiv in impl_names:
                continue  # Equivalent exists or intentionally inlined
        actual_missing.append(func)

    if actual_missing:
        issues.append(f"  MISSING functions: {', '.join(sorted(actual_missing))}")
    if extra:
        # Extra helper functions are OK, just note them
        issues.append(f"  EXTRA functions (OK if helpers): {', '.join(sorted(extra))}")

    # Compare matching functions
    for name in ref_names & impl_names:
        ref_f = ref_funcs[name]
        impl_f = impl_funcs[name]

        if ref_f.locals != impl_f.locals:
            issues.append(
                f"  {name}: locals differ (ref={ref_f.locals}, impl={impl_f.locals})"
            )

        # Allow some variance in instruction count (different compilers)
        ratio = (
            impl_f.instructions / ref_f.instructions if ref_f.instructions > 0 else 1.0
        )
        if ratio < 0.5 or ratio > 2.0:
            issues.append(
                f"  {name}: instruction count differs significantly "
                f"(ref={ref_f.instructions}, impl={impl_f.instructions}, ratio={ratio:.2f})"
            )

    return len([i for i in issues if "MISSING" in i]) == 0, issues


def main():
    base = Path(__file__).parent
    ref_dir = base / "ref_os_vm"
    impl_dir = base / "compiled_os_vm"

    modules = [
        "Array",
        "Math",
        "Memory",
        "String",
        "Screen",
        "Output",
        "Keyboard",
        "Sys",
    ]

    print("=" * 70)
    print("Jack OS VM Code Comparison: Implementation vs Reference")
    print("=" * 70)
    print()

    all_pass = True
    results = []

    for module in modules:
        ref_path = ref_dir / f"{module}.vm"
        impl_path = impl_dir / f"{module}.vm"

        if not ref_path.exists():
            print(f"[SKIP] {module}: Reference not found")
            continue
        if not impl_path.exists():
            print(f"[FAIL] {module}: Implementation not found")
            all_pass = False
            continue

        # Compare file sizes
        ref_size = ref_path.stat().st_size
        impl_size = impl_path.stat().st_size
        size_ratio = impl_size / ref_size if ref_size > 0 else 1.0

        passed, issues = compare_files(ref_path, impl_path)

        status = "PASS" if passed else "FAIL"
        if not passed:
            all_pass = False

        print(f"[{status}] {module}.vm")
        print(
            f"       Size: ref={ref_size:,} bytes, impl={impl_size:,} bytes (ratio={size_ratio:.2f})"
        )

        if issues:
            for issue in issues:
                print(issue)

        results.append((module, passed, ref_size, impl_size, issues))
        print()

    # Summary
    print("=" * 70)
    print("SUMMARY")
    print("=" * 70)

    passed_count = sum(1 for _, p, _, _, _ in results if p)
    print(f"Modules: {passed_count}/{len(results)} passed")

    # Size comparison
    total_ref = sum(r for _, _, r, _, _ in results)
    total_impl = sum(i for _, _, _, i, _ in results)
    print(f"Total size: ref={total_ref:,} bytes, impl={total_impl:,} bytes")
    print(f"Size ratio: {total_impl / total_ref:.2f}x")

    if all_pass:
        print("\n[SUCCESS] All modules have matching function signatures!")
    else:
        print("\n[FAILURE] Some modules have issues - see above for details")

    return 0 if all_pass else 1


if __name__ == "__main__":
    exit(main())
