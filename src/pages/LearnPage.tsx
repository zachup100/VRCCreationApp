import { useActions } from "../actions";
import { Icon } from "../components/Icons";
import type { IconName } from "../components/Icons";

const links: { title: string; blurb: string; url: string; icon: IconName }[] = [
	{ title: "VRChat Creators", blurb: "Official guides for building worlds and avatars.", url: "https://creators.vrchat.com", icon: "book" },
	{ title: "Creator Companion Docs", blurb: "How VPM packages, repositories and projects work.", url: "https://vcc.docs.vrchat.com", icon: "package" },
	{ title: "Download Unity", blurb: "Get Unity Hub and the editor versions VRChat supports.", url: "https://unity.com/download", icon: "unity" },
	{ title: "vrc-get on GitHub", blurb: "The open-source package manager this app builds on.", url: "https://github.com/vrc-get/vrc-get", icon: "external" },
];

export function LearnPage() {
	const actions = useActions();
	return (
		<div className="page">
			<header className="page-header">
				<h1>Learn</h1>
			</header>
			<div className="grid">
				{links.map((link) => (
					<button key={link.url} type="button" className="card link-card" onClick={() => void actions.openLink(link.url)}>
						<Icon name={link.icon} size={28} />
						<strong>{link.title}</strong>
						<span>{link.blurb}</span>
						<Icon name="external" size={16} className="link-corner" />
					</button>
				))}
			</div>
		</div>
	);
}
