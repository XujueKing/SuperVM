import matplotlib.pyplot as plt
import matplotlib
matplotlib.rcParams['font.sans-serif'] = ['SimHei', 'Microsoft YaHei']
matplotlib.rcParams['axes.unicode_minus'] = False

# 鏁版嵁
chains = ['Bitcoin', 'Ethereum', 'Cardano', 'Solana', 'Visa', 'SuperVM']
tps = [7, 15, 250, 50000, 65000, 242000]
colors = ['#F7931A', '#627EEA', '#0033AD', '#14F195', '#1A1F71', '#E74C3C']

# 鍒涘缓鍥捐〃
fig, ax = plt.subplots(figsize=(12, 7))
bars = ax.bar(chains, tps, color=colors, edgecolor='black', linewidth=1.5)

# 娣诲姞鏁板€兼爣绛?
for i, bar in enumerate(bars):
    height = bar.get_height()
    label = f'{int(height):,} TPS' if height < 1000 else f'{int(height/1000)}K TPS'
    ax.text(bar.get_x() + bar.get_width()/2., height,
            label,
            ha='center', va='bottom', fontsize=11, fontweight='bold')

# 楂樹寒 SuperVM
bars[-1].set_edgecolor('#8B0000')
bars[-1].set_linewidth(4)

# 鏍峰紡
ax.set_ylabel('TPS (姣忕浜ゆ槗鏁?', fontsize=13, fontweight='bold')
ax.set_title('鍖哄潡閾炬€ц兘瀵规瘮\\nSuperVM: 242,000 TPS 瀹炴祴鎬ц兘', 
             fontsize=16, fontweight='bold', pad=20)
ax.set_yscale('log')
ax.grid(axis='y', alpha=0.3, linestyle='--')
ax.set_ylim(1, 500000)

# 娣诲姞娉ㄩ噴
ax.annotate('姣?Ethereum 蹇玕\n16,133 鍊?, 
            xy=(5, 242000), xytext=(4.5, 100000),
            arrowprops=dict(arrowstyle='->', color='red', lw=2),
            fontsize=11, color='red', fontweight='bold',
            bbox=dict(boxstyle='round,pad=0.5', facecolor='yellow', alpha=0.7))

plt.tight_layout()
plt.savefig('visuals/charts/performance-comparison.png', dpi=300, bbox_inches='tight')
print('Done: 鎬ц兘瀵规瘮鍥惧凡鐢熸垚: visuals/charts/performance-comparison.png')
