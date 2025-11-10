import matplotlib.pyplot as plt
import matplotlib
matplotlib.rcParams['font.sans-serif'] = ['SimHei', 'Microsoft YaHei']
matplotlib.rcParams['axes.unicode_minus'] = False

# 鏁版嵁
chains = ['Ethereum', 'BSC', 'Polygon', 'Arbitrum', 'Optimism', 'SuperVM']
gas_usd = [15.30, 0.50, 0.05, 0.80, 0.60, 0.01]
colors = ['#627EEA', '#F3BA2F', '#8247E5', '#28A0F0', '#FF0420', '#E74C3C']

# 鍒涘缓鍥捐〃
fig, ax = plt.subplots(figsize=(12, 7))
bars = ax.bar(chains, gas_usd, color=colors, edgecolor='black', linewidth=1.5)

# 娣诲姞鏁板€兼爣绛?
for i, bar in enumerate(bars):
    height = bar.get_height()
    ax.text(bar.get_x() + bar.get_width()/2., height,
            f'\${height:.2f}',
            ha='center', va='bottom', fontsize=12, fontweight='bold')

# 楂樹寒 SuperVM
bars[-1].set_edgecolor('#8B0000')
bars[-1].set_linewidth(4)

# 鏍峰紡
ax.set_ylabel('Gas 璐圭敤 (USD)', fontsize=13, fontweight='bold')
ax.set_title('璺ㄩ摼 Gas 璐圭敤瀵规瘮\\nSuperVM: 姣?Ethereum 渚垮疁 99.3%', 
             fontsize=16, fontweight='bold', pad=20)
ax.set_ylim(0, max(gas_usd) * 1.3)
ax.grid(axis='y', alpha=0.3, linestyle='--')

# 娣诲姞鑺傜渷鐧惧垎姣?
savings = ((gas_usd[0] - gas_usd[-1]) / gas_usd[0]) * 100
ax.text(5, gas_usd[0] * 0.8, f'鑺傜渷 {savings:.1f}%', 
        fontsize=14, color='red', fontweight='bold',
        bbox=dict(boxstyle='round,pad=0.8', facecolor='yellow', alpha=0.8))

plt.tight_layout()
plt.savefig('assets/visuals/charts/gas-comparison.png', dpi=300, bbox_inches='tight')
print('Done: gas fee chart saved: assets/visuals/charts/gas-comparison.png')
