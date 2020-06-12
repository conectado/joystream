import React from "react";
import { css } from "@emotion/core";

import { Carousel, theme } from "@joystream/components";

type VideoGalleryProps = {
	title?: string;
	children?: React.ReactNode;
};

const sectionStyles = css`
	margin-bottom: 2rem;
	padding: 1rem;

	& > h4 {
		font-size: ${theme.typography.sizes.h4};
		margin-block: 1rem;
	}
`;

export default function Gallery({ title = "", children }: VideoGalleryProps) {
	return (
		<section css={sectionStyles}>
			<h4>{title}</h4>
			<Carousel>{children}</Carousel>
		</section>
	);
}
