"""Mann-Whitney U (two-sided, normal approx. w/ tie correction) + Cliff's delta on final normalized fitness.
Lower NF is better. Usage: python experiments/bbob/evidence/stats.py  (stdlib only)"""
import json, math, collections
fin = collections.defaultdict(dict)
for line in open('experiments/bbob/evidence/results.jsonl'):
    r = json.loads(line); t = fin[(r['function'], r['condition'])]; cur = t.get(r['trial'])
    if cur is None or r['tick'] >= cur[0]: t[r['trial']] = (r['tick'], r['normalized_fitness'])
def mwu(a, b):
    n1, n2 = len(a), len(b); allv = sorted([(v, 0) for v in a] + [(v, 1) for v in b])
    ranks = [0.0]*len(allv); i = 0; tie = 0.0
    while i < len(allv):
        j = i
        while j+1 < len(allv) and allv[j+1][0] == allv[i][0]: j += 1
        r = (i+j)/2 + 1
        for k in range(i, j+1): ranks[k] = r
        tt = j-i+1; tie += tt**3 - tt; i = j+1
    R1 = sum(r for r, (_, g) in zip(ranks, allv) if g == 0)
    U1 = R1 - n1*(n1+1)/2; N = n1+n2; mu = n1*n2/2
    sd = math.sqrt(n1*n2/12 * ((N+1) - tie/(N*(N-1))))
    if sd == 0: return 1.0, 0.0
    z = (U1-mu)/sd; p = math.erfc(abs(z)/math.sqrt(2))
    delta = (2*U1 - n1*n2)/(n1*n2)          # Cliff's delta, a=T1_T4 vs b=T1_T5: >0 means T1_T4 larger (worse)
    return p, delta
print("function         median_T1-T4  median_T1-T5   p (MWU)    Cliff_d (T4>T5 => T5 better)")
for fn in ['f1_sphere','f2_ellipsoid','f15_rastrigin','f24_lunacek']:
    a = [v for _, v in fin[(fn,'T1_T4')].values()]; b = [v for _, v in fin[(fn,'T1_T5')].values()]
    med = lambda x: sorted(x)[len(x)//2] if len(x)%2 else sum(sorted(x)[len(x)//2-1:len(x)//2+1])/2
    p, d = mwu(a, b)
    print(f"{fn:<16} {med(a):>11.4f} {med(b):>13.4f}   {p:<9.4g}  {d:+.2f}")
