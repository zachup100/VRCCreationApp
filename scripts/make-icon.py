def P(pts): return " ".join(f"{x:.1f},{y:.1f}" for x,y in pts)
def cube(cx,cy,s,top,left,right,rot=0):
    w=s*0.866; h=s*0.5
    T=(cx,cy-s); R=(cx+w,cy-h); B=(cx,cy); L=(cx-w,cy-h)
    BR=(cx+w,cy+h); BL=(cx-w,cy+h); D=(cx,cy+s)
    o=f'<g transform="rotate({rot} {cx} {cy})">'
    o+=f'<polygon points="{P([L,B,D,BL])}" fill="{left}"/>'
    o+=f'<polygon points="{P([B,R,BR,D])}" fill="{right}"/>'
    o+=f'<polygon points="{P([T,R,B,L])}" fill="{top}"/>'
    return o+'</g>'
def sparkle(x,y,r,fill="#fff"):
    k=r*0.18
    return f'<path d="M{x},{y-r} L{x+k},{y-k} L{x+r},{y} L{x+k},{y+k} L{x},{y+r} L{x-k},{y+k} L{x-r},{y} L{x-k},{y-k}Z" fill="{fill}"/>'

# box geometry
T=(512,560);R=(712,675);B=(512,790);L=(312,675)
H=170
D=(512,790+H);BL=(312,675+H);BR=(712,675+H)
# inner back walls
IT=(512,560)
s=[]
s.append('''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<defs>
<linearGradient id="bg" x1="0.1" y1="0" x2="0.9" y2="1">
<stop offset="0" stop-color="#5b2fd1"/><stop offset="0.55" stop-color="#2c1a7a"/><stop offset="1" stop-color="#150b3d"/></linearGradient>
<radialGradient id="glowbg" cx="0.5" cy="0.62" r="0.55">
<stop offset="0" stop-color="#ff5fc8" stop-opacity="0.55"/><stop offset="0.5" stop-color="#b13cff" stop-opacity="0.18"/><stop offset="1" stop-color="#b13cff" stop-opacity="0"/></radialGradient>
<linearGradient id="beam" x1="0" y1="1" x2="0" y2="0">
<stop offset="0" stop-color="#9ffcff" stop-opacity="0.95"/><stop offset="0.6" stop-color="#5fe0ff" stop-opacity="0.35"/><stop offset="1" stop-color="#5fe0ff" stop-opacity="0"/></linearGradient>
<radialGradient id="well" cx="0.5" cy="0.5" r="0.5">
<stop offset="0" stop-color="#ffffff"/><stop offset="0.45" stop-color="#9ffcff"/><stop offset="1" stop-color="#2fb8ff"/></radialGradient>
<linearGradient id="sheen" x1="0" y1="0" x2="1" y2="1">
<stop offset="0" stop-color="#fff" stop-opacity="0.22"/><stop offset="0.4" stop-color="#fff" stop-opacity="0"/></linearGradient>
<clipPath id="sq"><rect x="40" y="40" width="944" height="944" rx="230"/></clipPath>
<filter id="blur12" x="-50%" y="-200%" width="200%" height="500%"><feGaussianBlur stdDeviation="12"/></filter>
<filter id="blur30" x="-50%" y="-200%" width="200%" height="500%"><feGaussianBlur stdDeviation="30"/></filter>
</defs>
<rect x="40" y="40" width="944" height="944" rx="230" fill="url(#bg)"/>
<g clip-path="url(#sq)">
<rect x="40" y="40" width="944" height="944" fill="url(#glowbg)"/>
''')
# faint grid floor lines (blueprint) 
g=[]
for i in range(-6,7):
    g.append(f'<line x1="{512+i*130}" y1="1000" x2="{512+i*40}" y2="760" stroke="#c9a7ff" stroke-opacity="0.10" stroke-width="3"/>')
for k,y in enumerate([800,850,915,990]):
    g.append(f'<line x1="40" y1="{y}" x2="984" y2="{y}" stroke="#c9a7ff" stroke-opacity="0.10" stroke-width="3"/>')
s+=g
# stars dots
import random
random.seed(7)
for _ in range(26):
    x=random.randint(90,930);y=random.randint(90,520)
    if abs(x-512)<150 and y>150: continue
    s.append(f'<circle cx="{x}" cy="{y}" r="{random.choice([2,3,4])}" fill="#fff" fill-opacity="{random.choice([0.25,0.4,0.6])}"/>')
# beam
s.append(f'<polygon points="{P([(392,640),(632,640),(700,110),(324,110)])}" fill="url(#beam)" opacity="0.8"/>')
s.append(f'<polygon points="{P([(452,640),(572,640),(600,160),(424,160)])}" fill="url(#beam)" opacity="0.5"/>')
# floor shadow/glow under box
s.append(f'<ellipse cx="512" cy="960" rx="300" ry="40" fill="#0a0420" opacity="0.5" filter="url(#blur12)"/>')

# floating island
ix,iy=512,330
top=[(ix,iy-70),(ix+150,iy-5),(ix,iy+60),(ix-150,iy-5)]
s.append(f'<ellipse cx="{ix}" cy="{iy+10}" rx="190" ry="70" fill="#7fffd4" opacity="0.35" filter="url(#blur30)"/>')
# underside facets
tip=(ix+6,iy+185)
s.append(f'<polygon points="{P([top[3],top[2],tip])}" fill="#5a3a2a"/>')
s.append(f'<polygon points="{P([top[2],top[1],tip])}" fill="#3f261c"/>')
s.append(f'<polygon points="{P([top[3],(ix-60,iy+35),(ix-20,iy+95),tip])}" fill="#74503a"/>')
s.append(f'<polygon points="{P([(ix+20,iy+60),top[1],(ix+70,iy+70),tip])}" fill="#5a3a2a" opacity="0.8"/>')
# grass facets
s.append(f'<polygon points="{P([top[3],top[0],top[2]])}" fill="#7dff9a"/>')
s.append(f'<polygon points="{P([top[0],top[1],top[2]])}" fill="#3fd47a"/>')
s.append(f'<polygon points="{P([top[3],(ix-60,iy-40),(ix-20,iy-10)])}" fill="#a8ffb8" opacity="0.7"/>')
s.append(f'<polygon points="{P([(ix+40,iy+5),(ix+100,iy-5),(ix+60,iy+40)])}" fill="#2bb866"/>')
# tree
tx,ty=ix-10,iy-5
s.append(f'<polygon points="{P([(tx-9,ty),(tx+9,ty),(tx+9,ty-40),(tx-9,ty-40)])}" fill="#8a5a3a"/>')
s.append(f'<polygon points="{P([(tx,ty-150),(tx+52,ty-60),(tx,ty-45)])}" fill="#ffb64d"/>')
s.append(f'<polygon points="{P([(tx,ty-150),(tx-52,ty-60),(tx,ty-45)])}" fill="#ffd27a"/>')
s.append(f'<polygon points="{P([(tx,ty-115),(tx+40,ty-40),(tx,ty-28)])}" fill="#ff8f3d"/>')
s.append(f'<polygon points="{P([(tx,ty-115),(tx-40,ty-40),(tx,ty-28)])}" fill="#ffb64d"/>')
# crystal on island
cx2,cy2=ix+80,iy+5
s.append(f'<polygon points="{P([(cx2,cy2-50),(cx2+16,cy2-14),(cx2,cy2+6),(cx2-16,cy2-14)])}" fill="#ff7be0"/>')
s.append(f'<polygon points="{P([(cx2,cy2-50),(cx2+16,cy2-14),(cx2,cy2+6)])}" fill="#ff3fc0"/>')

# floating cubes
s.append(cube(300,430,46,"#ffe27a","#ff9d3d","#ff6a3d",-8))
s.append(cube(735,470,34,"#a8fff2","#32c7d9","#1d8fc4",10))
s.append(cube(640,250,20,"#ffd1f3","#ff7be0","#c93fb0",0))
s.append(cube(385,235,16,"#c9fff0","#46e0b8","#25a58c",0))

# box: interior
s.append(f'<polygon points="{P([T,R,B,L])}" fill="#0b0624"/>')
s.append(f'<polygon points="{P([T,R,B,L])}" fill="url(#well)" opacity="0.0"/>')
s.append(f'<ellipse cx="512" cy="675" rx="150" ry="62" fill="url(#well)"/>')
# inner back wall shading

# outside faces
s.append(f'<polygon points="{P([L,B,D,BL])}" fill="#ff7a59"/>')
s.append(f'<polygon points="{P([B,R,BR,D])}" fill="#d63a8a"/>')
# tape / panel decals on faces: circuit-like package tape
lf=[L,B,D,BL]
s.append(f'<polygon points="{P([(372,705),(432,740),(432,740+H),(372,705+H)])}" fill="#ffd27a" opacity="0.0"/>')
# front faces highlight edge
s.append(f'<polyline points="{P([L,B,R])}" fill="none" stroke="#fff0c9" stroke-width="10" stroke-linejoin="round" stroke-linecap="round"/>')
s.append(f'<polyline points="{P([T,L])}" fill="none" stroke="#ffb18f" stroke-width="10" stroke-linecap="round"/>')
s.append(f'<polyline points="{P([T,R])}" fill="none" stroke="#f080b8" stroke-width="10" stroke-linecap="round"/>')
s.append(f'<line x1="512" y1="790" x2="512" y2="{790+H}" stroke="#7a1f55" stroke-width="8" stroke-linecap="round"/>')
# side face glyphs: vpm brackets-like chevrons made of small parallelograms (left face)
def lface(u,v): # u along B->L (0..1), v down 0..1
    x=512+(312-512)*u; y=790+(675-790)*u+H*v
    return (x,y)
def rface(u,v):
    x=512+(712-512)*u; y=790+(675-790)*u+H*v
    return (x,y)
# left face: three stacked package-slot bars
for i,(a,b) in enumerate([(0.18,0.8),(0.18,0.55),(0.18,0.68)]):
    pass
bars=[(0.2,0.78,0.22,0.32),(0.2,0.55,0.48,0.58)]
def quad(f,u0,u1,v0,v1): return P([f(u0,v0),f(u1,v0),f(u1,v1),f(u0,v1)])
s.append(f'<polygon points="{quad(lface,0.18,0.82,0.22,0.34)}" fill="#fff0c9" opacity="0.9"/>')
s.append(f'<polygon points="{quad(lface,0.18,0.55,0.46,0.58)}" fill="#fff0c9" opacity="0.6"/>')
s.append(f'<polygon points="{quad(lface,0.18,0.68,0.70,0.82)}" fill="#fff0c9" opacity="0.35"/>')
# right face: node graph (dependency tree)
n=[rface(0.30,0.30),rface(0.72,0.28),rface(0.5,0.62),rface(0.78,0.74)]
for a,b in [(0,2),(1,2),(2,3)]:
    s.append(f'<line x1="{n[a][0]:.1f}" y1="{n[a][1]:.1f}" x2="{n[b][0]:.1f}" y2="{n[b][1]:.1f}" stroke="#ffd1ee" stroke-width="7" stroke-linecap="round"/>')
for i,(x,y) in enumerate(n):
    s.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="15" fill="{"#ffe27a" if i==2 else "#fff0f9"}"/>')
# lip glow rim light on top of box
s.append(f'<ellipse cx="512" cy="675" rx="215" ry="70" fill="#9ffcff" opacity="0.18" filter="url(#blur12)"/>')
# sparkles
s.append(sparkle(745,235,52))
s.append(sparkle(290,300,30,"#ffe9a8"))
s.append(sparkle(800,330,22,"#bff"))
s.append(sparkle(235,560,24))
s.append(sparkle(800,600,18,"#ffd1f3"))
s.append(sparkle(512,590,34,"#fff"))
s.append('<rect x="40" y="40" width="944" height="944" fill="url(#sheen)"/>')
s.append('</g></svg>')
import os,subprocess,tempfile
root=os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
svg=os.path.join(tempfile.gettempdir(),"vrc-app-icon.svg")
open(svg,"w").write("\n".join(s))
subprocess.run(["rsvg-convert","-w","1024","-h","1024",svg,"-o",os.path.join(root,"app-icon.png")],check=True)
